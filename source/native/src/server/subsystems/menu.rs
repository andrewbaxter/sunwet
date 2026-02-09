use {
    crate::{
        interface::config::{
            MenuItemPage,
            ServerConfigMenuItem,
            ServerConfigMenuItemDetail,
        },
        server::{
            access::Identity,
            state::{
                get_global_config,
                get_iam_grants,
                GlobalConfig,
                IamGrants,
                State,
            },
        },
    },
    loga::Log,
    shared::interface::config::{
        form::ClientForm,
        view::ClientView,
        ClientConfig,
        ClientFormLink,
        ClientMenuItem,
        ClientMenuItemDetail,
        ClientMenuSection,
        ClientPage,
        ClientViewLink,
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
    fn compile_visible_menu(
        log: &Log,
        config: &GlobalConfig,
        iam_grants: &IamGrants,
        at: &ServerConfigMenuItem,
        has_access: bool,
    ) -> Result<Option<ClientMenuItem>, loga::Error> {
        let has_access = has_access || match iam_grants {
            IamGrants::Admin => {
                true
            },
            IamGrants::Limited(g) => {
                if g.menu_items.contains(&at.id) {
                    true
                } else {
                    false
                }
            },
        };
        let out_detail = match &at.detail {
            ServerConfigMenuItemDetail::Section(d) => {
                let mut children = vec![];
                for at_child in &d.children {
                    let Some(child) = compile_visible_menu(log, config, iam_grants, at_child, has_access)? else {
                        continue;
                    };
                    children.push(child);
                }
                if children.is_empty() {
                    return Ok(None);
                }
                ClientMenuItemDetail::Section(ClientMenuSection { children: children })
            },
            ServerConfigMenuItemDetail::Page(d) => {
                if !has_access {
                    return Ok(None);
                }
                match d {
                    MenuItemPage::View(d) => {
                        ClientMenuItemDetail::Page(ClientPage::View(ClientViewLink {
                            view_id: d.view_id.clone(),
                            parameters: d.parameters.clone(),
                        }))
                    },
                    MenuItemPage::Form(d) => {
                        ClientMenuItemDetail::Page(ClientPage::Form(ClientFormLink {
                            form_id: d.form_id.clone(),
                            parameters: d.parameters.clone(),
                        }))
                    },
                    MenuItemPage::History => {
                        ClientMenuItemDetail::Page(ClientPage::History)
                    },
                    MenuItemPage::Query => {
                        ClientMenuItemDetail::Page(ClientPage::Query)
                    },
                    MenuItemPage::Logs => {
                        ClientMenuItemDetail::Page(ClientPage::Logs)
                    },
                    MenuItemPage::Offline => {
                        ClientMenuItemDetail::Page(ClientPage::Offline)
                    },
                }
            },
        };
        return Ok(Some(ClientMenuItem {
            id: at.id.clone(),
            name: at.name.clone(),
            detail: out_detail,
        }));
    }

    let iam_grants = get_iam_grants(&state, identity).await?;
    let global_config = get_global_config(&state).await?;
    let mut menu = vec![];
    for root_id in &global_config.menu {
        let Some(item) = compile_visible_menu(&state.log, &global_config, &iam_grants, root_id, false)? else {
            continue;
        };
        menu.push(item);
    }
    let mut forms = HashMap::new();
    for (k, form) in &global_config.forms {
        forms.insert(k.clone(), ClientForm {
            fields: form.item.fields.clone(),
            outputs: form.item.outputs.clone(),
        });
    }
    let mut views = HashMap::new();
    for (k, view) in &global_config.views {
        views.insert(k.clone(), ClientView {
            root: view.item.display.clone(),
            parameter_specs: view.item.parameters.clone(),
            query_parameter_keys: view.query_parameters.clone(),
            shuffle: view.shuffle,
        });
    }
    return Ok(ClientConfig {
        menu: menu,
        forms: forms,
        views: views,
    });
}
