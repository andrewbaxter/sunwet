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
    flowcontrol::shed,
    shared::interface::config::{
        form::ClientForm,
        menu::{
            ClientMenuItem,
            ClientMenuItemForm,
            ClientMenuItemSection,
            ClientMenuItemView,
        },
        ClientConfig,
        ClientView,
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
                return Some(ClientMenuItem::Section(ClientMenuItemSection {
                    name: at.name.clone(),
                    children: children,
                }));
            },
            crate::server::state::MenuItem::View(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                views.entry(at.item.view_id.clone()).or_insert_with(|| {
                    let view = config.views.get(&at.item.view_id).unwrap();
                    return ClientView { config: view.layout.clone() };
                });
                return Some(ClientMenuItem::View(ClientMenuItemView {
                    name: at.item.name.clone(),
                    id: at_id.clone(),
                    arguments: at.item.arguments.clone(),
                }));
            },
            crate::server::state::MenuItem::Form(at) => {
                if !iam_grants.match_set(&at.self_and_ancestors) {
                    return None;
                }
                forms.entry(at.item.form_id.clone()).or_insert_with(|| {
                    let form = config.forms.get(&at.item.form_id).unwrap();
                    return ClientForm {
                        fields: form.fields.clone(),
                        outputs: form.outputs.clone(),
                    };
                });
                return Some(ClientMenuItem::Form(ClientMenuItemForm {
                    name: at.item.name.clone(),
                    id: at_id.clone(),
                }));
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
    return Ok(ClientConfig {
        menu: menu,
        views: views,
        forms: forms,
    });
}
