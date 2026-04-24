use {
    gloo::utils::window,
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
            trigger_onlining,
        },
        world::scan_env,
    },
    std::rc::Rc,
    sunwet_browser::{
        capture_button::init_app_state,
        site_twitter::build_twitter,
    },
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
    trigger_onlining(&state, eg, &log, &env.base_url);
    if let Ok(host) = window().location().hostname() {
        let host = host.split_once(":").map(|x| x.0).unwrap_or(&host);
        match host {
            "x.com" => {
                build_twitter();
            },
            _ => { },
        }
    }
}
