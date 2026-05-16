use {
    lunk::{
        EventGraph,
        Prim,
    },
    shared_wasm::{
        log::{
            ConsoleLog,
            Log,
        },
        online::{
            OnliningState,
            trigger_onlining_no_lock,
        },
    },
    std::rc::Rc,
    sunwet_browser::{
        get_setting,
        init_app_state,
        KEY_SERVER_URL,
    },
    wasm_bindgen_futures::spawn_local,
};

fn main() {
    let state = Rc::new(OnliningState {
        bg: Default::default(),
        running: Prim::new(false),
    });
    let log: Rc<dyn Log> = Rc::new(ConsoleLog {});
    let eg = EventGraph::new();
    init_app_state(state.clone(), eg.clone(), log.clone());
    spawn_local(async move {
        let Some(base_url) = get_setting(KEY_SERVER_URL).await else {
            log.log("sunwet: no server URL configured, skipping onlining");
            return;
        };
        let base_url = if base_url.ends_with('/') {
            base_url
        } else {
            format!("{}/", base_url)
        };
        trigger_onlining_no_lock(&state, eg, &log, &base_url);
    });
}
