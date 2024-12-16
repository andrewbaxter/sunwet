use {
    super::state::State,
    crate::serverlib::query::QueryAccess,
    http::Response,
    htwrap::htserve::responses::{
        response_200_json,
        Body,
    },
    shared::interface::{
        config::menu::{
            MenuItem,
            MenuItemSection,
        },
        wire::GetMenuResp,
    },
    std::sync::Arc,
};

pub async fn handle_get_menu(state: Arc<State>, access: &QueryAccess) -> Result<Response<Body>, loga::Error> {
    fn compile_visible_menu(access: &QueryAccess, items: &[MenuItem]) -> Vec<MenuItem> {
        let mut out = vec![];
        for item in items {
            match item {
                MenuItem::Section(i) => {
                    if access.contains(i.iam_target) {
                        out.push(MenuItem::Section(MenuItemSection {
                            iam_target: i.iam_target,
                            name: i.name.clone(),
                            children: compile_visible_menu(access, &i.children),
                        }));
                    }
                },
                MenuItem::View(i) => {
                    if access.contains(i.iam_target) {
                        out.push(item.clone());
                    }
                },
                MenuItem::Form(i) => {
                    if access.contains(i.iam_target) {
                        out.push(item.clone());
                    }
                },
            }
        }
        return out;
    }

    return Ok(response_200_json(compile_visible_menu(access, &state.config.menu) as GetMenuResp));
}
