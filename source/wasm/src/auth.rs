use {
    crate::ministate::{
        ministate_octothorpe,
        read_ministate,
        Ministate,
    },
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::window,
    },
    web_sys::Url,
};

pub const LOCALSTORAGE_LOGGED_IN: &str = "want_logged_in";

pub fn want_logged_in() -> bool {
    return LocalStorage::get::<String>(LOCALSTORAGE_LOGGED_IN).is_ok();
}

pub fn redirect_login(base_url: &str) {
    window()
        .location()
        .set_href(
            &Url::new_with_base(
                &format!(
                    "oidc?url={}",
                    urlencoding::encode(
                        &format!(
                            "{}{}",
                            base_url,
                            ministate_octothorpe(&read_ministate().unwrap_or(Ministate::Home))
                        ),
                    )
                ),
                base_url,
            )
                .unwrap()
                .href(),
        )
        .unwrap();
}

pub fn redirect_logout(base_url: &str) {
    window()
        .location()
        .set_href(
            &Url::new_with_base(
                &format!(
                    "logout?url={}",
                    urlencoding::encode(
                        &format!(
                            "{}{}",
                            base_url,
                            ministate_octothorpe(&read_ministate().unwrap_or(Ministate::Home))
                        ),
                    )
                ),
                base_url,
            )
                .unwrap()
                .href(),
        )
        .unwrap();
}
