use {
    super::{
        ministate::{
            PlaylistEntryPath,
            PlaylistPos,
        },
        playlist::{
            ImagePlaylistMedia,
            PlaylistEntryMediaType,
        },
    },
    crate::{
        constants::LINK_HASH_PREFIX,
        el_general::{
            el_async,
            el_audio,
            el_video,
            log,
            style_export,
        },
        ont::{
            ROOT_AUDIO_VALUE,
            ROOT_IMAGE_VALUE,
            ROOT_VIDEO_VALUE,
        },
        playlist::{
            playlist_clear,
            playlist_len,
            playlist_next,
            playlist_previous,
            playlist_push,
            playlist_seek,
            playlist_toggle_play,
            AudioPlaylistMedia,
            PlaylistEntry,
            VideoPlaylistMedia,
        },
        state::{
            set_page,
            State,
        },
        util::OptString,
        websocket::Ws,
        world::{
            file_url,
            generated_file_url,
            req_post_json,
        },
    },
    chrono::{
        Duration,
        Utc,
    },
    flowcontrol::{
        shed,
        superif,
    },
    gloo::{
        timers::future::TimeoutFuture,
        utils::{
            format::JsValueSerdeExt,
            window,
        },
    },
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    qrcode::QrCode,
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
        ScopeValue,
    },
    shared::interface::{
        triple::{
            FileHash,
            Node,
        },
        wire::{
            ReqViewQuery,
            TreeNode,
        },
    },
    std::{
        cell::RefCell,
        collections::{
            BTreeMap,
            HashMap,
        },
        rc::Rc,
        str::FromStr,
    },
    uuid::Uuid,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        DomParser,
        Element,
        Event,
        HtmlElement,
        HtmlInputElement,
        HtmlMediaElement,
        MouseEvent,
        SupportedType,
    },
};

#[derive(Clone)]
pub struct BuildPlaylistPos {
    pub list_id: String,
    pub list_title: String,
    pub entry_path: Option<PlaylistEntryPath>,
}

impl BuildPlaylistPos {
    pub fn add(&self, a: Option<Node>) -> Self {
        return Self {
            list_id: self.list_id.clone(),
            list_title: self.list_title.clone(),
            entry_path: match (&self.entry_path, a) {
                (Some(ep), Some(a)) => {
                    let mut out = ep.0.clone();
                    out.push(a);
                    Some(PlaylistEntryPath(out))
                },
                _ => None,
            },
        };
    }
}

pub fn build_page_view(
    outer_state: &State,
    list_title: &str,
    list_id: &str,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) {
    set_page(outer_state, list_title, el_async({
        let outer_state = outer_state.clone();
        let view_id = list_id.to_string();
        async move {
            let client_config = outer_state.client_config.get().await?;
            let Some(view) = client_config.views.get(&view_id) else {
                return Err(format!("No view with id [{}] in config", view_id));
            };
            let view_el = JsFuture::from(style_export::build_view(style_export::BuildViewArgs {
                plugin_path: format!("{}.js", view_id),
                arguments: JsValue::from_serde(&view.config).unwrap(),
            }).root).await.map_err(|_| format!("Error building view, check browser console"))?;
            return Ok(el_from_raw(view_el.dyn_into::<Element>().unwrap()));
            ;
        }
    }));
}
