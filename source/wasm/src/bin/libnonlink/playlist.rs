use {
    super::{
        ministate::PlaylistRestorePos,
        state::{
            state,
            MinistateViewState,
        },
    },
    crate::libnonlink::api::req_file,
    chrono::Utc,
    futures::FutureExt,
    gloo::{
        timers::{
            callback::Interval,
            future::TimeoutFuture,
        },
        utils::window,
    },
    js_sys::Function,
    lunk::{
        link,
        HistPrim,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el,
        scope_any,
        spawn_rooted,
        El,
    },
    serde::Deserialize,
    shared::interface::{
        derived::ComicManifest,
        wire::{
            link::{
                Prepare,
                PrepareAudio,
                PrepareMedia,
                SourceUrl,
                WsC2S,
                WsS2C,
            },
            GENTYPE_DIR,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::BTreeMap,
        ops::Bound,
        rc::{
            Rc,
            Weak,
        },
    },
    wasm::{
        js::Env,
        media::{
            pm_share_ready_prep,
            PlaylistMedia,
            PlaylistMediaAudioVideo,
            PlaylistMediaBook,
            PlaylistMediaComic,
            PlaylistMediaImage,
        },
        websocket::Ws,
        world::{
            file_url,
            generated_file_url,
        },
    },
    wasm_bindgen::{
        closure::Closure,
        JsCast,
        JsValue,
    },
    web_sys::{
        HtmlElement,
        HtmlMediaElement,
        MediaMetadata,
    },
};

pub type PlaylistIndex = Vec<usize>;

#[derive(Deserialize, Clone, Copy)]
pub enum PlaylistEntryMediaType {
    Audio,
    Video,
    Image,
    Comic,
    Book,
}

pub struct PlaylistEntry {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub cover_source_url: Option<SourceUrl>,
    pub source_url: SourceUrl,
    pub media_type: PlaylistEntryMediaType,
    pub media: Box<dyn PlaylistMedia>,
    pub play_buttons: Vec<HtmlElement>,
}

pub struct PlaylistState_ {
    pub view_ministate_state: RefCell<Option<MinistateViewState>>,
    pub env: Env,
    pub playlist: RefCell<BTreeMap<PlaylistIndex, Rc<PlaylistEntry>>>,
    pub playing: HistPrim<bool>,
    // Must be Some if playing, otherwise may be Some.
    pub playing_i: HistPrim<Option<PlaylistIndex>>,
    pub media_time: Prim<f64>,
    pub playing_time: Prim<f64>,
    pub media_max_time: Prim<Option<f64>>,
    pub share: Prim<Option<(String, Ws<WsC2S, WsS2C>)>>,
    pub media_el_audio: El,
    pub media_el_video: El,
    pub media_el_image: El,
}

#[derive(Clone)]
pub struct PlaylistState(pub Rc<PlaylistState_>);

impl PlaylistState {
    pub fn weak(&self) -> WeakPlaylistState {
        return WeakPlaylistState(Rc::downgrade(&self.0));
    }

    pub fn format_time(&self, time: f64) -> String {
        let Some(playing_i) = &*self.0.playing_i.borrow() else {
            return format!("00:00");
        };
        let playlist = self.0.playlist.borrow();
        let entry = playlist.get(&*playing_i).unwrap();
        return entry.media.pm_format_time(time);
    }
}

#[derive(Clone)]
pub struct WeakPlaylistState(pub Weak<PlaylistState_>);

impl WeakPlaylistState {
    pub fn upgrade(&self) -> Option<PlaylistState> {
        match self.0.upgrade() {
            Some(p) => return Some(PlaylistState(p)),
            None => return None,
        }
    }
}

fn playlist_first_index(state: &PlaylistState) -> Option<PlaylistIndex> {
    return state.0.playlist.borrow().first_key_value().map(|x| x.0.clone());
}

pub fn state_new(pc: &mut ProcessingContext, env: Env) -> (PlaylistState, rooting::ScopeValue) {
    let setup_media_element = |pc: &mut ProcessingContext, media: &El| {
        media.ref_on("ended", {
            let eg = pc.eg();
            move |_| eg.event(|pc| {
                playlist_next(pc, &state().playlist, None);
            }).unwrap()
        });
        media.ref_on("loadedmetadata", {
            move |ev| {
                let state = state();
                let ministate = state.playlist.0.view_ministate_state.borrow();
                let Some(ministate) = &*ministate else {
                    return;
                };
                let ministate = ministate.0.borrow();
                let Some(restore) = ministate.pos.as_ref() else {
                    return;
                };
                if state.playlist.0.playing_i.get().as_ref() != Some(&restore.index) {
                    return;
                }
                ev.target().unwrap().dyn_into::<HtmlMediaElement>().unwrap().set_current_time(restore.time);
            }
        });
    };
    let media_el_audio = el("audio").attr("preload", "metadata").attr("controls", "true");
    setup_media_element(pc, &media_el_audio);
    let media_el_video = el("video").attr("preload", "metadata").attr("controls", "true");
    setup_media_element(pc, &media_el_video);
    let media_el_image = el("img").attr("loading", "lazy");
    let playlist_state = PlaylistState(Rc::new(PlaylistState_ {
        env: env.clone(),
        playlist: RefCell::new(Default::default()),
        playing: HistPrim::new(pc, false),
        playing_i: HistPrim::new(pc, None),
        playing_time: Prim::new(0.),
        media_time: Prim::new(0.),
        media_max_time: Prim::new(None),
        view_ministate_state: Default::default(),
        share: Prim::new(None),
        media_el_audio: media_el_audio,
        media_el_video: media_el_video,
        media_el_image: media_el_image,
    }));
    let media_session = window().navigator().media_session();

    // # Media control
    fn media_fn(pc: &mut ProcessingContext, f: impl 'static + Fn(&mut ProcessingContext, JsValue) -> ()) -> Function {
        let eg = pc.eg();
        let fn1 =
            Closure::<dyn Fn(JsValue) -> ()>::wrap(Box::new(move |args| eg.event(|pc| f(pc, args)).unwrap()));
        let fn2: Function = fn1.as_ref().unchecked_ref::<Function>().to_owned();
        fn1.forget();
        return fn2;
    }

    media_session.set_action_handler(web_sys::MediaSessionAction::Play, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, _args| {
            playlist_resume(pc, &state);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Pause, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, _args| {
            playlist_pause(pc, &state);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Stop, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, _args| {
            state.0.playing.set(pc, false);
            state.0.playing_i.set(pc, None);
            state.0.media_max_time.set(pc, None);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Nexttrack, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, _args| {
            playlist_next(pc, &state, state.0.playing_i.get());
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Previoustrack, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, _args| {
            playlist_previous(pc, &state, state.0.playing_i.get());
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekforward, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, args| {
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let new_time = *state.0.playing_time.borrow() + offset;
            state.0.playing_time.set(pc, new_time);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekbackward, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, args| {
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let new_time = (*state.0.playing_time.borrow() - offset).max(0.);
            state.0.playing_time.set(pc, new_time);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekto, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, args| {
            let time = js_sys::Reflect::get(&args, &JsValue::from("seekTime")).unwrap().as_f64().unwrap();
            playlist_seek(pc, &state, time);
        }
    })));
    let bg = Rc::new(Cell::new(None));
    return (playlist_state.clone(), scope_any((
        //. .
        // Play, pause, track switch
        link!(
            //. .
            (pc = pc),
            (playing = playlist_state.0.playing.clone(), playing_i = playlist_state.0.playing_i.clone()),
            (),
            (playlist_state = playlist_state.clone(), media_session = media_session, bg = bg.clone()) {
                match playlist_state.0.playing_i.get() {
                    Some(i) => {
                        let e = playlist_state.0.playlist.borrow().get(&i).cloned().unwrap();
                        media_session.set_metadata(Some(&{
                            let m = MediaMetadata::new().unwrap();
                            if let Some(name) = &e.name {
                                m.set_title(name);
                            }
                            if let Some(album) = &e.album {
                                m.set_album(album);
                            }
                            if let Some(artist) = &e.artist {
                                m.set_artist(artist);
                            }
                            if let Some(cover) = &e.cover_source_url {
                                let arr = js_sys::Array::new();
                                let e = js_sys::Object::new();
                                js_sys::Reflect::set(&e, &JsValue::from("src"), &JsValue::from(&match cover {
                                    SourceUrl::Url(v) => v.clone(),
                                    SourceUrl::File(v) => file_url(&state().env, &v),
                                })).unwrap();
                                arr.push(e.dyn_ref().unwrap());
                                m.set_artwork(&arr.dyn_into().unwrap());
                            }
                            m
                        }));
                    },
                    None => {
                        media_session.set_metadata(None);
                    },
                }
                if !*playing.borrow() {
                    // Stop previous
                    if let Some(i) = playing_i.get_old() {
                        if let Some(e) = playlist_state.0.playlist.borrow().get(&i).cloned() {
                            if let Some((_, ws)) = &*playlist_state.0.share.borrow() {
                                bg.set(Some(spawn_rooted({
                                    let ws = ws.clone();
                                    async move {
                                        ws.send(WsC2S::Pause).await;
                                    }
                                })));
                            }
                            e.media.pm_stop();
                        }
                    }
                } else {
                    // Stop previous if it changed
                    if let Some(i) = playing_i.get_old().as_ref() {
                        if Some(i) != playing_i.get().as_ref() {
                            let e = playlist_state.0.playlist.borrow().get(i).cloned().unwrap();
                            e.media.pm_stop();
                            e.media.pm_seek(pc, 0.);
                            e.media.pm_unpreload();
                        }
                    }

                    // Start next/current
                    let i = match playing_i.get() {
                        Some(i) => i,
                        None => playlist_first_index(playlist_state).unwrap(),
                    };
                    let e = playlist_state.0.playlist.borrow().get(&i).cloned().unwrap();
                    if let Some((_, ws)) = &*playlist_state.0.share.borrow() {
                        bg.set(Some(spawn_rooted({
                            let ws = ws.clone();
                            let eg = pc.eg();
                            let env = playlist_state.0.env.clone();
                            async move {
                                let new_time = e.media.pm_get_time();
                                ws.send(WsC2S::Prepare(Prepare {
                                    artist: e.artist.clone().unwrap_or_default(),
                                    album: e.album.clone().unwrap_or_default(),
                                    name: e.name.clone().unwrap_or_default(),
                                    media: match e.media_type {
                                        PlaylistEntryMediaType::Audio => PrepareMedia::Audio(PrepareAudio {
                                            cover_source_url: e.cover_source_url.clone(),
                                            source_url: e.source_url.clone(),
                                        }),
                                        PlaylistEntryMediaType::Video => PrepareMedia::Video(e.source_url.clone()),
                                        PlaylistEntryMediaType::Image => PrepareMedia::Image(e.source_url.clone()),
                                        PlaylistEntryMediaType::Comic => PrepareMedia::Comic(e.source_url.clone()),
                                        PlaylistEntryMediaType::Book => PrepareMedia::Book(e.source_url.clone()),
                                    },
                                    media_time: e.media.pm_get_time(),
                                })).await;
                                pm_share_ready_prep(eg, &env, e.media.as_ref(), new_time).await;
                                ws.send(WsC2S::Ready(Utc::now())).await;
                            }
                        })));
                    } else {
                        e.media.pm_preload(&playlist_state.0.env);
                        e.media.pm_play();
                    }
                }
            }
        ),
        // Progression of time, from media element
        Interval::new(1000, {
            let state = playlist_state.clone();
            let eg = pc.eg();
            let last_state = Cell::new(None);
            move || {
                let Some(playing_i) = &*state.0.playing_i.borrow() else {
                    return;
                };
                let time;
                let max_time;
                {
                    let playlist = state.0.playlist.borrow();
                    let entry: &Rc<PlaylistEntry> = playlist.get(&*playing_i).unwrap();
                    time = entry.media.pm_get_time();
                    max_time = entry.media.pm_get_max_time();
                }
                let new_state = (time, max_time);
                if Some(&new_state) == last_state.get().as_ref() {
                    return;
                }
                last_state.set(Some(new_state));
                eg.event(|pc| {
                    state.0.media_time.set(pc, time);
                    state.0.media_max_time.set(pc, max_time);
                });
                if let Some(vs) = state.0.view_ministate_state.borrow().as_ref() {
                    vs.set_pos(Some(PlaylistRestorePos {
                        index: playing_i.clone(),
                        time: time,
                    }));
                }
            }
        }),
        // Sync progression of time back
        link!(
            //. .
            (pc = pc),
            (media_time = playlist_state.0.media_time.clone()),
            (playing_time = playlist_state.0.playing_time.clone()),
            () {
                playing_time.set(pc, *media_time.borrow());
            }
        ),
        // Seek
        link!(
            //. .
            (pc = pc),
            (playing_time = playlist_state.0.playing_time.clone()),
            (media_time = playlist_state.0.media_time.clone()),
            (playlist_state = playlist_state.clone(), bg = bg.clone()) {
                let new_time = *playing_time.borrow();
                media_time.set(pc, new_time);
                if *playlist_state.0.playing.borrow() {
                    let i = playlist_state.0.playing_i.get().unwrap();
                    let e = playlist_state.0.playlist.borrow().get(&i).cloned().unwrap();
                    let env = playlist_state.0.env.clone();
                    if let Some((_, ws)) = &*playlist_state.0.share.borrow() {
                        e.media.pm_stop();
                        bg.set(Some(spawn_rooted({
                            let ws = ws.clone();
                            let eg = pc.eg();
                            async move {
                                ws.send(WsC2S::Prepare(Prepare {
                                    artist: e.artist.clone().unwrap_or_default(),
                                    album: e.album.clone().unwrap_or_default(),
                                    name: e.name.clone().unwrap_or_default(),
                                    media: match e.media_type {
                                        PlaylistEntryMediaType::Audio => PrepareMedia::Audio(PrepareAudio {
                                            cover_source_url: e.cover_source_url.clone(),
                                            source_url: e.source_url.clone(),
                                        }),
                                        PlaylistEntryMediaType::Video => PrepareMedia::Video(e.source_url.clone()),
                                        PlaylistEntryMediaType::Image => PrepareMedia::Image(e.source_url.clone()),
                                        PlaylistEntryMediaType::Comic => PrepareMedia::Comic(e.source_url.clone()),
                                        PlaylistEntryMediaType::Book => PrepareMedia::Book(e.source_url.clone()),
                                    },
                                    media_time: new_time,
                                })).await;
                                pm_share_ready_prep(eg, &env, e.media.as_ref(), new_time).await;
                                ws.send(WsC2S::Ready(Utc::now())).await;
                            }
                        })));
                    } else {
                        e.media.pm_seek(pc, new_time);
                    }
                }
            }
        ),
    )));
}

pub fn playlist_set_link(pc: &mut ProcessingContext, playlist_state: &PlaylistState, id: &str) {
    playlist_state.0.share.set(pc, Some((id.to_string(), Ws::new(&state().env.base_url, format!("main/{}", id), {
        let playlist_state = playlist_state.clone();
        let bg = Cell::new(None);
        move |_, msg| {
            match msg {
                WsS2C::Play(play_at) => {
                    if !playlist_state.0.playing.get() {
                        return;
                    }
                    let i = match playlist_state.0.playing_i.get() {
                        Some(i) => i,
                        None => playlist_first_index(&playlist_state).unwrap(),
                    };
                    let e = playlist_state.0.playlist.borrow().get(&i).cloned().unwrap();
                    bg.set(Some(spawn_rooted({
                        let playlist_state = playlist_state.clone();
                        async move {
                            TimeoutFuture::new((play_at - Utc::now()).num_milliseconds().max(0) as u32).await;
                            if !playlist_state.0.playing.get() {
                                return;
                            }
                            e.media.pm_play();
                        }
                    })));
                },
            }
        }
    }))));
}

pub fn playlist_len(state: &PlaylistState) -> usize {
    return state.0.playlist.borrow().len();
}

pub struct PlaylistPushArg {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub cover_source_url: Option<SourceUrl>,
    pub source_url: SourceUrl,
    pub media_type: PlaylistEntryMediaType,
    pub play_buttons: Vec<HtmlElement>,
}

pub fn playlist_extend(
    pc: &mut ProcessingContext,
    playlist_state: &PlaylistState,
    vs: MinistateViewState,
    entries: Vec<(PlaylistIndex, PlaylistPushArg)>,
    restore_pos: &Option<PlaylistRestorePos>,
) {
    *playlist_state.0.view_ministate_state.borrow_mut() = Some(vs);
    for (entry_index, entry) in entries {
        let box_media: Box<dyn PlaylistMedia>;
        match entry.media_type {
            PlaylistEntryMediaType::Audio => {
                box_media =
                    Box::new(
                        PlaylistMediaAudioVideo::new_audio(
                            playlist_state.0.media_el_audio.clone(),
                            entry.source_url.clone(),
                        ),
                    );
            },
            PlaylistEntryMediaType::Video => {
                box_media =
                    Box::new(
                        PlaylistMediaAudioVideo::new_video(
                            playlist_state.0.media_el_video.clone(),
                            entry.source_url.clone(),
                        ),
                    );
            },
            PlaylistEntryMediaType::Image => {
                box_media = Box::new(PlaylistMediaImage {
                    element: playlist_state.0.media_el_image.clone(),
                    src: entry.source_url.clone(),
                });
            },
            PlaylistEntryMediaType::Comic => {
                box_media = Box::new(PlaylistMediaComic::new(&match &entry.source_url {
                    SourceUrl::Url(u) => u.to_string(),
                    SourceUrl::File(h) => generated_file_url(&state().env, h, GENTYPE_DIR, ""),
                }, Rc::new(|url| async move {
                    return Ok(
                        serde_json::from_slice::<ComicManifest>(
                            &req_file(&state().env.base_url, &url).await?,
                        ).map_err(|e| format!("Error reading comic manifest: {}", e))?,
                    );
                }.boxed_local()), 0));
            },
            PlaylistEntryMediaType::Book => {
                box_media = Box::new(PlaylistMediaBook::new(&match &entry.source_url {
                    SourceUrl::Url(u) => u.to_string(),
                    SourceUrl::File(h) => generated_file_url(&state().env, h, GENTYPE_DIR, ""),
                }, 0));
            },
        }
        if let Some(restore_pos) = restore_pos {
            if restore_pos.index == entry_index && !playlist_state.0.playing.get() {
                playlist_state.0.playing_i.set(pc, Some(entry_index.clone()));
            }
        }
        playlist_state.0.playlist.borrow_mut().insert(entry_index, Rc::new(PlaylistEntry {
            name: entry.name,
            album: entry.album,
            artist: entry.artist,
            cover_source_url: entry.cover_source_url,
            source_url: entry.source_url,
            media_type: entry.media_type,
            media: box_media,
            play_buttons: entry.play_buttons,
        }));
    }
}

pub fn playlist_clear(pc: &mut ProcessingContext, state: &PlaylistState) {
    if *state.0.playing.borrow() {
        let playing_i = state.0.playing_i.get().unwrap();
        let playlist = state.0.playlist.borrow();
        let entry = playlist.get(&*playing_i).unwrap();
        entry.media.pm_stop();
    }
    state.0.playing.set(pc, false);
    state.0.playing_i.set(pc, None);
    state.0.media_max_time.set(pc, None);
    state.0.playlist.borrow_mut().clear();
    *state.0.view_ministate_state.borrow_mut() = None;
}

pub fn playlist_toggle_play(pc: &mut ProcessingContext, state: &PlaylistState, i: Option<PlaylistIndex>) {
    if *state.0.playing.borrow() {
        let current_i = state.0.playing_i.get().unwrap();
        let i = i.as_ref().unwrap_or(&current_i);
        if &current_i == i {
            state.0.playing.set(pc, false);
        } else {
            state.0.playing_i.set(pc, Some(i.clone()));
            state.0.playing_time.set(pc, 0.);
        }
    } else {
        if state.0.playlist.borrow().is_empty() {
            return;
        }
        let i = i.or(state.0.playing_i.get()).unwrap_or(playlist_first_index(state).unwrap());
        if match &*state.0.playing_i.borrow() {
            Some(current_i) => *current_i != i,
            None => true,
        } {
            state.0.playing_time.set(pc, 0.);
        }
        state.0.playing_i.set(pc, Some(i));
        state.0.playing.set(pc, true);
    }
}

pub fn playlist_next(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<PlaylistIndex>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if let Some((i, _)) = state.0.playlist.borrow().range((Bound::Excluded(i), Bound::Unbounded)).next() {
        state.0.playing.set(pc, true);
        state.0.playing_i.set(pc, Some(i.clone()));
        state.0.playing_time.set(pc, 0.);
    } else {
        state.0.playing_i.set(pc, None);
        state.0.media_max_time.set(pc, None);
        state.0.playing.set(pc, false);
        state.0.playing_time.set(pc, 0.);
        if let Some(vs) = state.0.view_ministate_state.borrow().as_ref() {
            vs.set_pos(None);
        }
    }
}

pub fn playlist_previous(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<PlaylistIndex>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if let Some((i, _)) = state.0.playlist.borrow().range((Bound::Unbounded, Bound::Excluded(i))).rev().next() {
        state.0.playing_i.set(pc, Some(i.clone()));
        state.0.playing_time.set(pc, 0.);
    } else {
        state.0.playing_i.set(pc, None);
        state.0.media_max_time.set(pc, None);
        state.0.playing.set(pc, false);
        state.0.playing_time.set(pc, 0.);
    }
}

pub fn playlist_pause(pc: &mut ProcessingContext, state: &PlaylistState) {
    state.0.playing.set(pc, false);
}

pub fn playlist_resume(pc: &mut ProcessingContext, state: &PlaylistState) {
    if state.0.playlist.borrow().is_empty() {
        return;
    }
    if *state.0.playing.borrow() {
        return;
    }
    if state.0.playing_i.borrow().is_none() {
        state.0.playing_i.set(pc, Some(playlist_first_index(state).unwrap()));
    }
    state.0.playing.set(pc, true);
}

pub fn playlist_seek(pc: &mut ProcessingContext, state: &PlaylistState, time: f64) {
    state.0.playing_time.set(pc, time);
}
