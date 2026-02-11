use {
    crate::libnonlink::{
        api::{
            file_post_json,
            req_post_json,
        },
        commit::UploadFile,
        opfs::{
            opfs_root,
            request_persistent,
        },
        state::state,
    },
    chrono::Utc,
    flowcontrol::ta_return,
    gloo::{
        timers::future::TimeoutFuture,
        utils::window,
    },
    js_sys::Promise,
    lunk::EventGraph,
    rooting::{
        defer,
        spawn_rooted,
    },
    shared::interface::wire::{
        ReqCommit,
        ReqUploadFinish,
    },
    wasm::js::LogJsErr,
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

// # Outgoing
const OPFS_ONLINE_COMMIT_FILENAME: &str = "commit.json";
const OPFS_ONLINE_COMMIT_ROOT: &str = "online_commits";

pub async fn ensure_commit(eg: EventGraph, commit: ReqCommit, files: Vec<UploadFile>) -> Result<(), String> {
    request_persistent().await;
    let key = Utc::now().to_rfc3339();
    let commit_dir = opfs_root().await.ensure_dir(vec![OPFS_ONLINE_COMMIT_ROOT.to_string(), key]).await?;
    commit_dir.ensure_file(vec![OPFS_ONLINE_COMMIT_FILENAME.to_string()]).await?.write_json(&commit).await?;
    for file in files {
        commit_dir.ensure_file(vec![file.hash.to_string()]).await?.write_binary(&file.data).await?;
    }
    trigger_onlining(eg);
    return Ok(());
}

pub fn stop_onlining() {
    *state().onlining_bg.borrow_mut() = None;
}

pub fn trigger_onlining(eg: EventGraph) {
    let state1 = state();
    let mut bg = state1.onlining_bg.borrow_mut();
    if bg.is_none() {
        *bg = Some(spawn_rooted(async move {
            let eg = eg.clone();
            let cb = Closure::<dyn Fn(JsValue) -> Promise>::new(move |_| {
                let eg = eg.clone();
                return future_to_promise(async move {
                    eg.event(|pc| {
                        state().onlining.set(pc, true);
                    }).unwrap();
                    let _cleanup = defer({
                        let eg = eg.clone();
                        move || eg.event(|pc| {
                            *state().onlining_bg.borrow_mut() = None;
                            state().onlining.set(pc, false);
                        }).unwrap()
                    });
                    let commit_root = opfs_root().await.ensure_dir(vec![OPFS_ONLINE_COMMIT_ROOT.to_string()]).await?;
                    for (key, task_dir) in commit_root.list().await? {
                        let task_dir = match task_dir.dir() {
                            Ok(d) => d,
                            Err(e) => {
                                state().log.log(&e);
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
                            let need_files = req_post_json(req).await?;
                            for file in need_files.incomplete {
                                let data = task_dir.get_file(vec![file.to_string()]).await?.read_binary().await?;
                                const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
                                let file_size = data.len() as u64;
                                let chunks = file_size.div_ceil(CHUNK_SIZE);
                                for i in 0 .. chunks {
                                    let chunk_start = i * CHUNK_SIZE;
                                    let chunk_size = file_size.min(CHUNK_SIZE);
                                    file_post_json(
                                        &file,
                                        chunk_start,
                                        &data[chunk_start as usize .. (chunk_start + chunk_size) as usize],
                                    ).await?;
                                }
                                loop {
                                    let resp = req_post_json(ReqUploadFinish(file.clone())).await?;
                                    if resp.done {
                                        break;
                                    }
                                    TimeoutFuture::new(1000).await;
                                }
                            }
                            commit_root.delete(&key).await;
                            return Ok(());
                        }.await;
                        match res {
                            Ok(_) => { },
                            Err(e) => {
                                state()
                                    .log
                                    .log(&format!("Error uploading queued commit at [{}]: {}", task_dir.0, e));
                            },
                        }
                    }

                    // Nothing left to do atm, exit
                    eg.event(|pc| {
                        state().onlining.set(pc, false);
                    }).unwrap();
                    return Ok(JsValue::null());
                });
            });
            JsFuture::from(window().navigator().locks().request_with_callback("online", cb.as_ref().unchecked_ref()))
                .await
                .log(&state().log, "Error doing work in `online` lock");
        }));
    }
}
