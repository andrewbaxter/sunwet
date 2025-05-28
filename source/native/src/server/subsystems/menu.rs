use {
    crate::{
        interface::config::IamGrants,
        server::{
            access::Identity,
            state::{
                get_global_config,
                get_iam_grants,
                GlobalConfig,
                State,
            },
        },
    },
    flowcontrol::exenum,
    shared::interface::config::{
        form::ClientForm,
        view::ClientView,
        ClientConfig,
        ClientMenuItem,
        ClientMenuSection,
    },
    std::{
        collections::HashMap,
        sync::Arc,
    },
};

pub async fn handle_get_filtered_client_config(
    state: Arc<State>,
    identity: &Identity,
) -> Result<ClientConfig, loga::Error> {
    let mut views = HashMap::new();
    let mut forms = HashMap::new();

    fn compile_visible_menu(
        views: &mut HashMap<String, ClientView>,
        forms: &mut HashMap<String, ClientForm>,
        config: &GlobalConfig,
        iam_grants: &IamGrants,
        at_id: &String,
    ) -> Option<ClientMenuItem> {
        match config.menu_items.get(at_id).unwrap() {
            crate::server::state::MenuItem::Section(at) => {
                let mut children = vec![];
                for child_id in &at.children {
                    let Some(child) = compile_visible_menu(views, forms, config, iam_grants, child_id) else {
                        continue;
                    };
                    children.push(child);
                }
                if children.is_empty() {
                    return None;
                }
                return Some(ClientMenuItem::Section(ClientMenuSection {
                    name: at.name.clone(),
                    children: children,
                }));
            },
            crate::server::state::MenuItem::View(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                return Some(ClientMenuItem::View(ClientView {
                    id: at_id.clone(),
                    name: at.item.name.clone(),
                    root: at.item.root.clone(),
                    parameters: at.item.parameters.clone(),
                }));
            },
            crate::server::state::MenuItem::Form(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                return Some(ClientMenuItem::Form(ClientForm {
                    id: at_id.clone(),
                    name: at.item.name.clone(),
                    fields: at.item.fields.clone(),
                    outputs: at.item.outputs.clone(),
                }));
            },
            crate::server::state::MenuItem::History => {
                if exenum!(iam_grants, IamGrants:: Admin =>()).is_none() {
                    return None;
                }
                return Some(ClientMenuItem::History);
            },
        }
    }

    let iam_grants = get_iam_grants(&state, identity).await?;
    let global_config = get_global_config(&state).await?;
    let mut menu = vec![];
    for root_id in &global_config.menu {
        let Some(item) = compile_visible_menu(&mut views, &mut forms, &global_config, &iam_grants, root_id) else {
            continue;
        };
        menu.push(item);
    }
    return Ok(ClientConfig { menu: menu });
}
