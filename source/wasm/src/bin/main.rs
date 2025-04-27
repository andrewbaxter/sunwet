use {
    flowcontrol::superif,
    gloo::{
        utils::window,
    },
    lunk::{
        EventGraph,
    },
    std::{
        panic,
    },
    wasm_bindgen::{
        UnwrapThrowExt,
    },
    web::{
        constants::LINK_HASH_PREFIX,
        el_general::{
            get_dom_octothorpe,
        },
        main_link::main_link,
        main_nonlink::main_nonlink,
    },
};

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    eg.event(|pc| {
        let base_url;
        {
            let loc = window().location();
            base_url = format!("{}{}", loc.origin().unwrap_throw(), loc.pathname().unwrap_throw());
        }

        // Short circuit to link mode
        superif!({
            let Some(hash) = get_dom_octothorpe() else {
                break;
            };
            let Some(link_id) = hash.strip_prefix(LINK_HASH_PREFIX) else {
                break;
            };
            break 'is_link link_id.to_string();
        } link_id = 'is_link {
            main_link(pc, base_url, link_id);
            return;
        });
        main_nonlink(pc, base_url);
    });
}
