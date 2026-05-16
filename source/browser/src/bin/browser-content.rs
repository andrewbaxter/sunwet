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
        world::scan_env,
    },
    std::rc::Rc,
    sunwet_browser::init_app_state,
};

fn main() {
    let state = Rc::new(OnliningState {
        bg: Default::default(),
        running: Prim::new(false),
    });
    let log: Rc<dyn Log> = Rc::new(ConsoleLog {});
    let eg = EventGraph::new();
    let env = scan_env(&log);
    init_app_state(state.clone(), eg.clone(), log.clone());
    trigger_onlining_no_lock(&state, eg, &log, &env.base_url);
}
