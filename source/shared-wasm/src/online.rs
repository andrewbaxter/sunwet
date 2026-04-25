use {
    crate::{
        api::{
            file_post_json,
            req_post_json,
        },
        commit::UploadFile,
        log::{
            Log,
            LogJsErr,
        },
        opfs::{
            opfs_root,
            request_persistent,
        },
    },
    chrono::Utc,
    flowcontrol::ta_return,
    gloo::{
        timers::future::TimeoutFuture,
        utils::window,
    },
    js_sys::Promise,
    lunk::{
        EventGraph,
        Prim,
    },
    rooting::{
        ScopeValue,
        defer,
        spawn_rooted,
    },
    shared::interface::wire::{
        ReqCommit,
        ReqCommitForm,
        ReqUploadFinish,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        prelude::Closure,
    },
    wasm_bindgen_futures::{
        JsFuture,
        future_to_promise,
    },
};

pub struct OnliningState {
    pub bg: RefCell<Option<ScopeValue>>,
    pub running: Prim<bool>,
}

// # Outgoing
const OPFS_ONLINE_COMMIT_FILENAME: &str = "commit.json";
const OPFS_ONLINE_COMMIT_ROOT: &str = "online_commits";

pub async fn ensure_commit(
    state: &Rc<OnliningState>,
    eg: EventGraph,
    log: &Rc<dyn Log>,
    base_url: &String,
    commit: ReqCommit,
    files: Vec<UploadFile>,
) -> Result<(), String> {
    request_persistent(&log).await;
    let key = Utc::now().to_rfc3339();
    let commit_dir = opfs_root().await.ensure_dir(vec![OPFS_ONLINE_COMMIT_ROOT.to_string(), key]).await?;
    commit_dir.ensure_file(vec![OPFS_ONLINE_COMMIT_FILENAME.to_string()]).await?.write_json(&commit).await?;
    for file in files {
        commit_dir.ensure_file(vec![file.hash.to_string()]).await?.write_binary(&file.data).await?;
    }
    trigger_onlining(state, eg, log, base_url);
    return Ok(());
}

pub async fn ensure_form_commit(
    state: &Rc<OnliningState>,
    eg: EventGraph,
    log: &Rc<dyn Log>,
    base_url: &String,
    form: ReqCommitForm,
) -> Result<(), String> {
    ensure_commit(state, eg, log, base_url, ReqCommit::Form(form), vec![]).await
}

pub fn stop_onlining(state: &OnliningState) {
    *state.bg.borrow_mut() = None;
}

pub fn trigger_onlining(state: &Rc<OnliningState>, eg: EventGraph, log: &Rc<dyn Log>, base_url: &String) {
    let mut bg = state.bg.borrow_mut();
    if bg.is_none() {
        let state = Rc::downgrade(&state);
        let log = log.clone();
        let base_url = base_url.clone();
        *bg = Some(spawn_rooted(async move {
            let cb = Closure::<dyn Fn(JsValue) -> Promise>::new({
                let eg = eg.clone();
                let log = log.clone();
                move |_| {
                    let eg = eg.clone();
                    let log = log.clone();
                    let base_url = base_url.clone();
                    let state = state.clone();
                    return future_to_promise(async move {
                        eg.event(|pc| {
                            let Some(state) = state.upgrade() else {
                                return;
                            };
                            state.running.set(pc, true);
                        }).unwrap();
                        let _cleanup = defer({
                            let eg = eg.clone();
                            let state = state.clone();
                            move || eg.event(|pc| {
                                let Some(state) = state.upgrade() else {
                                    return;
                                };
                                *state.bg.borrow_mut() = None;
                                state.running.set(pc, false);
                            }).unwrap()
                        });
                        let commits_root =
                            opfs_root().await.ensure_dir(vec![OPFS_ONLINE_COMMIT_ROOT.to_string()]).await?;
                        for (key, task_dir) in commits_root.list(&log).await? {
                            let task_dir = match task_dir.dir() {
                                Ok(d) => d,
                                Err(e) => {
                                    log.log(&e);
                                    continue;
                                },
                            };
                            let res = async {
                                ta_return!((), String);
                                let req: ReqCommit =
                                    task_dir
                                        .get_file(vec![OPFS_ONLINE_COMMIT_FILENAME.to_string()])
                                        .await?
                                        .read_json()
                                        .await?;
                                let need_files = req_post_json(&log, &base_url, req).await?;
                                for file in need_files.incomplete {
                                    let data = task_dir.get_file(vec![file.to_string()]).await?.read_binary().await?;
                                    const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
                                    let file_size = data.len() as u64;
                                    let chunks = file_size.div_ceil(CHUNK_SIZE);
                                    for i in 0 .. chunks {
                                        let chunk_start = i * CHUNK_SIZE;
                                        let chunk_end = (chunk_start + CHUNK_SIZE).min(file_size);
                                        let chunk_size = chunk_end - chunk_start;
                                        file_post_json(
                                            &log,
                                            &base_url,
                                            &file,
                                            chunk_start,
                                            &data[chunk_start as usize .. (chunk_start + chunk_size) as usize],
                                        ).await?;
                                    }
                                    loop {
                                        let resp =
                                            req_post_json(&log, &base_url, ReqUploadFinish(file.clone())).await?;
                                        if resp.done {
                                            break;
                                        }
                                        TimeoutFuture::new(1000).await;
                                    }
                                }
                                commits_root.delete(&log, &key).await;
                                return Ok(());
                            }.await;
                            match res {
                                Ok(_) => { },
                                Err(e) => {
                                    log.log(&format!("Error uploading queued commit at [{}]: {}", task_dir.0, e));
                                },
                            }
                        }

                        // Nothing left to do atm, exit
                        return Ok(JsValue::null());
                    });
                }
            });
            JsFuture::from(window().navigator().locks().request_with_callback("online", cb.as_ref().unchecked_ref()))
                .await
                .log(&log, "Error doing work in `online` lock");
        }));
    }
}
