use {
    crate::server::state::State,
    crate::server::state::get_global_config,
    shared::interface::{
        config::menu::{
            MenuItem,
            MenuItemSection,
        },
        iam::IamTargetId,
    },
    std::sync::Arc,
};

pub async fn handle_get_menu(
    state: Arc<State>,
    view_restriction: Option<Vec<IamTargetId>>,
) -> Result<Vec<MenuItem>, loga::Error> {
    fn restricted(view_restriction: &Option<Vec<IamTargetId>>, allow_target: &Option<IamTargetId>) -> bool {
        if let Some(allow_target) = allow_target {
            // Allows non-admin
            if let Some(targets) = view_restriction.as_ref() {
                if !targets.contains(allow_target) {
                    return true;
                }
            }
            return false;
        } else {
            // Requires admin
            if view_restriction.is_some() {
                // Restricted - not admin
                return true;
            }
            return false;
        }
    }

    fn compile_visible_menu(view_restriction: &Option<Vec<IamTargetId>>, items: &[MenuItem]) -> Vec<MenuItem> {
        let mut out = vec![];
        for item in items {
            match item {
                MenuItem::Section(i) => {
                    if restricted(view_restriction, &i.allow_target) {
                        continue;
                    }
                    out.push(MenuItem::Section(MenuItemSection {
                        allow_target: i.allow_target,
                        name: i.name.clone(),
                        children: compile_visible_menu(view_restriction, &i.children),
                    }));
                },
                MenuItem::View(i) => {
                    if restricted(view_restriction, &i.allow_target) {
                        continue;
                    }
                    out.push(item.clone());
                },
                MenuItem::Form(i) => {
                    if restricted(view_restriction, &i.allow_target) {
                        continue;
                    }
                    out.push(item.clone());
                },
            }
        }
        return out;
    }

    let global_config = get_global_config(&state).await?;
    return Ok(compile_visible_menu(&view_restriction, &global_config.config.menu));
}
