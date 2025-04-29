use {
    super::{
        ministate::{
            PlaylistEntryPath,
            PlaylistPos,
        },
        state::{
            set_page,
            state,
        },
    },
    crate::libnonlink::playlist::{
        playlist_next,
        playlist_previous,
        playlist_seek,
        playlist_toggle_play,
    },
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    rooting::el_from_raw,
    shared::interface::triple::Node,
    uuid::Uuid,
    wasm::{
        constants::LINK_HASH_PREFIX,
        el_general::{
            el_async,
            style_export::{
                self,
                js_get,
            },
        },
        websocket::Ws,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Element,
        Event,
        HtmlElement,
        MouseEvent,
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
    pc: &mut ProcessingContext,
    list_title: &str,
    list_id: &str,
    build_playlist_pos: &BuildPlaylistPos,
    restore_playlist_pos: &Option<PlaylistPos>,
) {
    set_page(list_title, el_async({
        let view_id = list_id.to_string();
        async move {
            // # Async
            let client_config = state().client_config.get().await?;
            let Some(view) = client_config.views.get(&view_id) else {
                return Err(format!("No view with id [{}] in config", view_id));
            };
            let build_res = style_export::build_view(style_export::BuildViewArgs {
                plugin_path: format!("{}.js", view_id),
                arguments: <JsValue as gloo::utils::format::JsValueSerdeExt>::from_serde(&view.config).unwrap(),
            });
            let build_res =
                JsFuture::from(build_res.root)
                    .await
                    .map_err(|_| format!("Error building view, check browser console"))?;

            // # Sync, assembly
            let hover_time = Prim::new(None);

            fn get_mouse_pct(ev: &Event) -> (f64, f64, MouseEvent) {
                let element = ev.target().unwrap().dyn_into::<Element>().unwrap();
                let ev = ev.dyn_ref::<MouseEvent>().unwrap();
                let element_rect = element.get_bounding_client_rect();
                let percent_x =
                    ((ev.client_x() as f64 - element_rect.x()) / element_rect.width().max(0.001)).clamp(0., 1.);
                let percent_y =
                    ((ev.client_y() as f64 - element_rect.y()) / element_rect.width().max(0.001)).clamp(0., 1.);
                return (percent_x, percent_y, ev.clone());
            }

            fn get_mouse_time(ev: &Event) -> Option<f64> {
                let Some(max_time) = *state().playlist.0.playing_max_time.borrow() else {
                    return None;
                };
                let percent = get_mouse_pct(ev).0;
                return Some(max_time * percent);
            }

            let root = el_from_raw(js_get::<HtmlElement>(&build_res, "root").into());
            let want_transport = js_get::<bool>(&build_res, "want_transport");
            let mut children = vec![];
            if want_transport {
                let transport_res = style_export::cont_bar_view_transport();
                let button_share = el_from_raw(transport_res.button_share.into());
                button_share.ref_on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        let sess_id = state().playlist.0.share.borrow().as_ref().map(|x| x.0.clone());
                        let sess_id = match sess_id {
                            Some(sess_id) => {
                                sess_id.clone()
                            },
                            None => {
                                let id = Uuid::new_v4().to_string();
                                state()
                                    .playlist
                                    .0
                                    .share
                                    .set(
                                        pc,
                                        Some((id.clone(), Ws::new(format!("main/{}", id), |_, _| unreachable!()))),
                                    );
                                id
                            },
                        };
                        let link = format!("{}#{}{}", state().base_url, LINK_HASH_PREFIX, sess_id);
                        stack.ref_push(el_modal(pc, "Share", |pc, root| {
                            return vec![
                                //. .
                                el("a").classes(&["g_qr"]).attr("href", &link).push(el_from_raw(
                                    //. .
                                    DomParser::new()
                                        .unwrap()
                                        .parse_from_string(
                                            &QrCode::new(&link)
                                                .unwrap()
                                                .render::<qrcode::render::svg::Color>()
                                                .quiet_zone(false)
                                                .build(),
                                            SupportedType::ImageSvgXml,
                                        )
                                        .unwrap()
                                        .first_element_child()
                                        .unwrap(),
                                )),
                                el_button_icon_text(pc, ICON_NOSHARE, "Stop sharing", {
                                    let state = state.clone();
                                    let root = root.clone();
                                    move |pc| {
                                        let Some(root) = root.upgrade() else {
                                            return;
                                        };
                                        state().playlist.0.share.set(pc, None);
                                        root.ref_replace(vec![]);
                                    }
                                })
                            ];
                        }));
                    }).unwrap()
                });

                // Prev
                let button_prev = el_from_raw(transport_res.button_prev.into());
                button_prev.ref_on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        playlist_previous(pc, &state().playlist, None);
                    }).unwrap()
                });

                // Next
                let button_next = el_from_raw(transport_res.button_next.into());
                button_next.ref_on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        playlist_next(pc, &state().playlist, None);
                    }).unwrap()
                });

                // Play
                let button_play = el_from_raw(transport_res.button_play.into());
                button_play.ref_on("click", {
                    let eg = pc.eg();
                    move |_| eg.event(|pc| {
                        playlist_toggle_play(pc, &state().playlist, None);
                    }).unwrap()
                });

                // Seekbar
                let seekbar = el_from_raw(transport_res.seekbar.into());
                seekbar.ref_on("mousemove", {
                    let eg = pc.eg();
                    let hover_time = hover_time.clone();
                    move |ev| eg.event(|pc| {
                        hover_time.set(pc, get_mouse_time(ev));
                    }).unwrap()
                });
                seekbar.ref_on("mouseleave", {
                    let eg = pc.eg();
                    let hover_time = hover_time.clone();
                    move |_| eg.event(|pc| {
                        hover_time.set(pc, None);
                    }).unwrap()
                });
                seekbar.ref_on("click", {
                    let eg = pc.eg();
                    move |ev| eg.event(|pc| {
                        let Some(time) = get_mouse_time(ev) else {
                            return;
                        };
                        playlist_seek(pc, &state().playlist, time);
                    }).unwrap()
                });
                let seekbar_fill = el_from_raw(transport_res.seekbar_fill.into());
                seekbar_fill.ref_own(|fill| link!(
                    //. .
                    (_pc = pc),
                    (
                        time = state().playlist.0.playing_time.clone(),
                        max_time = state().playlist.0.playing_max_time.clone(),
                    ),
                    (),
                    (fill = fill.weak()) {
                        let Some(max_time) = *max_time.borrow() else {
                            return None;
                        };
                        let fill = fill.upgrade()?;
                        fill.ref_attr(
                            "style",
                            &format!("width: {}%;", *time.borrow() / max_time.max(0.0001) * 100.),
                        );
                    }
                ));
                let seekbar_label = el_from_raw(transport_res.seekbar_label.into());
                seekbar_label.ref_own(|label| link!(
                    //. .
                    (_pc = pc),
                    (playing_time = state().playlist.0.playing_time.clone(), hover_time = hover_time.clone()),
                    (),
                    (label = label.weak()) {
                        let label = label.upgrade()?;
                        let time: f64;
                        if let Some(t) = *hover_time.borrow() {
                            time = t;
                        } else {
                            time = *playing_time.borrow();
                        }
                        let time = time as u64;
                        let seconds = time % 60;
                        let time = time / 60;
                        let minutes = time % 60;
                        let time = time / 60;
                        let hours = time % 24;
                        let days = time / 24;
                        if days > 0 {
                            label.text(&format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds));
                        } else if hours > 0 {
                            label.text(&format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
                        } else {
                            label.text(&format!("{:02}:{:02}", minutes, seconds));
                        }
                    }
                ));

                // Assemble
                children.push(
                    el_from_raw(
                        transport_res.root.into(),
                    ).own(|_| (button_share, button_prev, button_next, button_play, seekbar, seekbar_fill)),
                );
            }
            children.push(root);
            return Ok(el_from_raw(build_res.dyn_into::<Element>().unwrap()));
        }
    }));
}
