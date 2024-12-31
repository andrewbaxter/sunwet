use {
    crate::{
        interface::config::{
            IamGrants,
            PageAccess,
        },
        server::{
            access::Identity,
            state::{
                get_global_config,
                get_iam_grants,
                State,
            },
        },
    },
    shared::interface::config::menu::{
        MenuItem,
        MenuItemSection,
    },
    std::sync::Arc,
};

pub async fn handle_get_menu(state: Arc<State>, identity: &Identity) -> Result<Vec<MenuItem>, loga::Error> {
    fn compile_visible_menu(iam_grants: &IamGrants, items: &[MenuItem]) -> Vec<MenuItem> {
        let mut out = vec![];
        for item in items {
            match item {
                MenuItem::Section(i) => {
                    let children = compile_visible_menu(iam_grants, &i.children);
                    if !children.is_empty() {
                        out.push(MenuItem::Section(MenuItemSection {
                            name: i.name.clone(),
                            children: children,
                        }));
                    }
                },
                MenuItem::View(i) => {
                    match iam_grants {
                        IamGrants::Admin => { },
                        IamGrants::Limited(grants) => {
                            if !grants.contains(&PageAccess::View(i.id.clone())) {
                                continue;
                            }
                        },
                    }
                    out.push(item.clone());
                },
                MenuItem::Form(i) => {
                    match iam_grants {
                        IamGrants::Admin => { },
                        IamGrants::Limited(grants) => {
                            if !grants.contains(&PageAccess::Form(i.id.clone())) {
                                continue;
                            }
                        },
                    }
                    out.push(item.clone());
                },
            }
        }
        return out;
    }

    let iam_grants = get_iam_grants(&state, identity).await?;
    let global_config = get_global_config(&state).await?;
    return Ok(compile_visible_menu(&iam_grants, &global_config.config.menu));
}
