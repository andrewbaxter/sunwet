use {
    gloo::utils::window,
    lunk::EventGraph,
    shared_wasm::{
        log::ConsoleLog,
        online::{
            OnliningState,
            trigger_onlining,
        },
        world::scan_env,
    },
    std::rc::Rc,
    sunwet_browser::site_twitter::build_twitter,
};

fn main() {
    let state = Rc::new(OnliningState {
        bg: Default::default(),
        running: Prim::new(false),
    });
    let log = Rc::new(ConsoleLog);
    let eg = EventGraph::new();
    let env = scan_env(&log);
    trigger_onlining(&state, eg, &log, &env.base_url);
    if let Ok(host) = window().location().hostname() {
        let host = host.split_once(":").map(|x| x.0).unwrap_or(&host);
        match host {
            "x.com" => {
                build_twitter();
            },
        }
    }
}
