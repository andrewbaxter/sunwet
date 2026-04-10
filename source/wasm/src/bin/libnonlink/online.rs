use {
    crate::libnonlink::state::state,
    lunk::EventGraph,
    shared::interface::wire::ReqCommit,
    shared_wasm::{
        commit::UploadFile,
        online,
    },
};

pub async fn ensure_commit(eg: EventGraph, commit: ReqCommit, files: Vec<UploadFile>) -> Result<(), String> {
    return online::ensure_commit(
        &state().onlining_state,
        eg,
        &state().log,
        &state().env.base_url,
        commit,
        files,
    ).await;
}

pub fn trigger_onlining(eg: EventGraph) {
    online::trigger_onlining(&state().onlining_state, eg, &state().log, &state().env.base_url);
}

pub fn stop_onlining() {
    online::stop_onlining(&state().onlining_state);
}
