use {
    crate::libnonlink::{
        api::{
            file_post_json,
            req_post_json,
        },
        commit::UploadFile,
        opfs::{
            opfs_delete,
            opfs_ensure_dir,
            opfs_list_dir,
            opfs_read_binary,
            opfs_read_json,
            opfs_root,
            opfs_write_binary,
            opfs_write_json,
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
    rooting::spawn_rooted,
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
    web_sys::{
        ConnectionType,
        FileSystemDirectoryHandle,
    },
};

// # Outgoing
const OPFS_ONLINE_COMMIT_FILENAME: &str = "commit.json";

async fn opfs_commits_root() -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(&opfs_root().await, "online_commits").await;
}

pub async fn ensure_commit(eg: EventGraph, commit: ReqCommit, files: Vec<UploadFile>) -> Result<(), String> {
    let key = Utc::now().to_rfc3339();
    let commit_dir = opfs_ensure_dir(&opfs_commits_root().await, &key).await;
    opfs_write_json(&commit_dir, OPFS_ONLINE_COMMIT_FILENAME, &commit).await?;
    for file in files {
        opfs_write_binary(&commit_dir, &file.hash.to_string(), &file.data).await?;
    }
    trigger_onlining(eg);
    return Ok(());
}

pub fn trigger_onlining(eg: EventGraph) {
    let go = if let Ok(c) = window().navigator().connection() {
        match c.type_() {
            ConnectionType::Cellular | ConnectionType::None => {
                false
            },
            _ => {
                true
            },
        }
    } else {
        true
    };
    if go {
        *state().onlining_bg.borrow_mut() = None;
    } else {
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
                        let commit_root = opfs_commits_root().await;
                        for (key, task_dir) in opfs_list_dir(&commit_root).await {
                            let task_dir = match task_dir.dyn_into::<FileSystemDirectoryHandle>() {
                                Ok(d) => d,
                                Err(e) => {
                                    state()
                                        .log
                                        .log_js(
                                            &format!(
                                                "Found non-directory entry in upload root at [{}], deleting and continuing",
                                                key
                                            ),
                                            &e,
                                        );
                                    opfs_delete(&commit_root, &key).await;
                                    continue;
                                },
                            };
                            let res = async {
                                ta_return!((), String);
                                let req: ReqCommit = opfs_read_json(&commit_root, OPFS_ONLINE_COMMIT_FILENAME).await?;
                                let need_files = req_post_json(req).await?;
                                for file in need_files.incomplete {
                                    let data = opfs_read_binary(&task_dir, &file.to_string()).await?;
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
                                opfs_delete(&commit_root, &key).await;
                                return Ok(());
                            }.await;
                            match res {
                                Ok(_) => { },
                                Err(e) => {
                                    state()
                                        .log
                                        .log(
                                            &format!(
                                                "Error uploading queued commit [at opfs {}]: {}",
                                                task_dir.to_string().as_string().unwrap(),
                                                e
                                            ),
                                        );
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
                JsFuture::from(
                    window().navigator().locks().request_with_callback("online", cb.as_ref().unchecked_ref()),
                )
                    .await
                    .log(&state().log, "Error doing work in `online` lock");
            }));
        }
    }
}
