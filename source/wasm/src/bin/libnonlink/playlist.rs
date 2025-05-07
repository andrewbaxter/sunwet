use {
    super::{
        ministate::{
            Ministate,
            MinistateView,
            PlaylistRestorePos,
        },
        state::state,
    },
    crate::libnonlink::ministate::record_replace_ministate,
    chrono::{
        DateTime,
        Duration,
        Utc,
    },
    futures::{
        Future,
        FutureExt,
    },
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
    shared::interface::wire::link::{
        Prepare,
        PrepareAudio,
        PrepareMedia,
        SourceUrl,
        WsC2S,
        WsS2C,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::BTreeMap,
        ops::Bound,
        pin::Pin,
        rc::{
            Rc,
            Weak,
        },
    },
    wasm::{
        js::{
            async_event,
            el_audio,
            el_video,
            log,
            LogJsErr,
        },
        websocket::Ws,
        world::generated_file_url,
    },
    wasm_bindgen::{
        closure::Closure,
        JsCast,
        JsValue,
    },
    web_sys::{
        HtmlMediaElement,
        MediaMetadata,
    },
};

pub type PlaylistIndex = Vec<usize>;

pub trait PlaylistMedia {
    fn pm_display(&self) -> bool;
    fn pm_play(&self);
    fn pm_stop(&self);

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let time = self.pm_get_time();
        self.pm_seek(time + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let time = self.pm_get_time();
        self.pm_seek(time - offset_seconds);
    }
    fn pm_get_time(&self) -> f64;
    fn pm_get_max_time(&self) -> Option<f64>;
    fn pm_seek(&self, time: f64);
    fn pm_preload(&self);
    fn pm_unpreload(&self);
    fn pm_el(&self) -> &El;
    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct AudioPlaylistMedia {
    pub element: El,
}

impl AudioPlaylistMedia {
    fn pm_media(&self) -> HtmlMediaElement {
        return self.element.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for AudioPlaylistMedia {
    fn pm_display(&self) -> bool {
        return false;
    }

    fn pm_el(&self) -> &El {
        return &self.element;
    }

    fn pm_play(&self) {
        let audio = self.pm_media();
        audio.play().log("Error playing audio");
    }

    fn pm_stop(&self) {
        let audio = self.pm_media();
        audio.pause().unwrap();
    }

    fn pm_get_max_time(&self) -> Option<f64> {
        let audio = self.pm_media();
        let out = audio.duration();
        if !out.is_finite() {
            return None;
        } else {
            return Some(out);
        }
    }

    fn pm_get_time(&self) -> f64 {
        return self.pm_media().current_time();
    }

    fn pm_seek(&self, time: f64) {
        self.pm_media().set_current_time(time);
    }

    fn pm_preload(&self) {
        self.element.ref_attr("preload", "auto");
    }

    fn pm_unpreload(&self) {
        self.element.ref_attr("preload", "metadata");
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.pm_media().clone();
        return async move {
            // `HAVE_ENOUGH_DATA`
            if m.ready_state() < 4 {
                async_event(&m, "canplaythrough").await;
            }
        }.boxed_local();
    }
}

pub struct VideoPlaylistMedia {
    pub element: El,
}

impl VideoPlaylistMedia {
    fn pm_media(&self) -> HtmlMediaElement {
        return self.element.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for VideoPlaylistMedia {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self) -> &El {
        return &self.element;
    }

    fn pm_play(&self) {
        let s = self.pm_media();
        s.play().log("Error playing video");
    }

    fn pm_stop(&self) {
        let s = self.pm_media();
        s.pause().unwrap();
    }

    fn pm_get_max_time(&self) -> Option<f64> {
        let s = self.pm_media();
        let out = s.duration();
        if !out.is_finite() {
            return None;
        } else {
            return Some(out);
        }
    }

    fn pm_get_time(&self) -> f64 {
        return self.pm_media().current_time();
    }

    fn pm_seek(&self, time: f64) {
        self.pm_media().set_current_time(time);
    }

    fn pm_preload(&self) {
        self.element.ref_attr("preload", "auto");
    }

    fn pm_unpreload(&self) {
        self.element.ref_attr("preload", "metadata");
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.pm_media().clone();
        return async move {
            // `HAVE_ENOUGH_DATA`
            if m.ready_state() < 4 {
                async_event(&m, "canplaythrough").await;
            }
        }.boxed_local();
    }
}

pub struct ImagePlaylistMedia {
    pub element: El,
}

impl ImagePlaylistMedia { }

impl PlaylistMedia for ImagePlaylistMedia {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self) -> &El {
        return &self.element;
    }

    fn pm_play(&self) { }

    fn pm_stop(&self) { }

    fn pm_get_max_time(&self) -> Option<f64> {
        return None;
    }

    fn pm_get_time(&self) -> f64 {
        return 0.;
    }

    fn pm_seek(&self, _time: f64) { }

    fn pm_preload(&self) {
        self.element.ref_attr("loading", "eager");
    }

    fn pm_unpreload(&self) {
        self.element.ref_attr("loading", "auto");
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

#[derive(Deserialize, Clone, Copy)]
pub enum PlaylistEntryMediaType {
    Audio,
    Video,
    Image,
}

pub struct PlaylistEntry {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub cover_source_url: Option<SourceUrl>,
    pub source_url: SourceUrl,
    pub media_type: PlaylistEntryMediaType,
    pub media: Box<dyn PlaylistMedia>,
}

pub struct PlaylistState_ {
    pub ministate_menu_item_id_title: RefCell<Option<(String, String)>>,
    pub base_url: String,
    pub debounce: Cell<DateTime<Utc>>,
    pub playlist: RefCell<BTreeMap<PlaylistIndex, Rc<PlaylistEntry>>>,
    pub playing: HistPrim<bool>,
    // Must be Some if playing, otherwise may be Some.
    pub playing_i: HistPrim<Option<PlaylistIndex>>,
    pub playing_time: Prim<f64>,
    pub playing_max_time: Prim<Option<f64>>,
    pub share: Prim<Option<(String, Ws<WsC2S, WsS2C>)>>,
}

#[derive(Clone)]
pub struct PlaylistState(pub Rc<PlaylistState_>);

impl PlaylistState {
    pub fn weak(&self) -> WeakPlaylistState {
        return WeakPlaylistState(Rc::downgrade(&self.0));
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

pub fn state_new(pc: &mut ProcessingContext, base_url: String) -> (PlaylistState, rooting::ScopeValue) {
    let playlist_state = PlaylistState(Rc::new(PlaylistState_ {
        debounce: Cell::new(Utc::now()),
        base_url: base_url,
        playlist: RefCell::new(Default::default()),
        playing: HistPrim::new(pc, false),
        playing_i: HistPrim::new(pc, None),
        playing_time: Prim::new(0.),
        playing_max_time: Prim::new(None),
        ministate_menu_item_id_title: RefCell::new(None),
        share: Prim::new(None),
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
            state.0.playing_max_time.set(pc, None);
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
        move |_pc, args| {
            let Some(i) = state.0.playing_i.get() else {
                return;
            };
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let pm = state.0.playlist.borrow().get(&i).cloned().unwrap();
            pm.media.pm_seek_forward(offset);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekbackward, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |_pc, args| {
            let Some(i) = state.0.playing_i.get() else {
                return;
            };
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let pm = state.0.playlist.borrow().get(&i).cloned().unwrap();
            pm.media.pm_seek_backwards(offset);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekto, Some(&media_fn(pc, {
        let state = playlist_state.clone();
        move |pc, args| {
            let time = js_sys::Reflect::get(&args, &JsValue::from("seekTime")).unwrap().as_f64().unwrap();
            playlist_seek(pc, &state, time);
        }
    })));
    return (playlist_state.clone(), scope_any((
        //. .
        link!(
            //. .
            (_pc = pc),
            (playing = playlist_state.0.playing.clone(), playing_i = playlist_state.0.playing_i.clone()),
            (),
            (playlist_state = playlist_state.clone(), media_session = media_session, bg = Cell::new(None)) {
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
                                js_sys::Reflect::set(
                                    &e,
                                    &JsValue::from("src"),
                                    &JsValue::from(&cover.url),
                                ).unwrap();
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
                            playlist_state.0.debounce.set(Utc::now());
                            e.media.pm_stop();
                        }
                    }
                } else {
                    // Stop previous if it changed
                    if let Some(i) = playing_i.get_old().as_ref() {
                        if Some(i) != playing_i.get().as_ref() {
                            let e = playlist_state.0.playlist.borrow().get(i).cloned().unwrap();
                            playlist_state.0.debounce.set(Utc::now());
                            e.media.pm_stop();
                            e.media.pm_seek(0.);
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
                                    },
                                    media_time: e.media.pm_get_time(),
                                })).await;
                                e.media.pm_preload();
                                e.media.pm_wait_until_buffered().await;
                                ws.send(WsC2S::Ready(Utc::now())).await;
                            }
                        })));
                    } else {
                        playlist_state.0.debounce.set(Utc::now());
                        e.media.pm_play();
                    }
                }
            }
        ),
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
                    state.0.playing_time.set(pc, time);
                    state.0.playing_max_time.set(pc, max_time);
                });
                if let Some((menu_item_id, title)) = state.0.ministate_menu_item_id_title.borrow().as_ref() {
                    record_replace_ministate(&Ministate::View(MinistateView {
                        menu_item_id: menu_item_id.clone(),
                        title: title.clone(),
                        pos: Some(PlaylistRestorePos {
                            index: playing_i.clone(),
                            time: time,
                        }),
                    }));
                }
            }
        }),
    )));
}

pub fn playlist_set_link(pc: &mut ProcessingContext, playlist_state: &PlaylistState, id: &str) {
    playlist_state.0.share.set(pc, Some((id.to_string(), Ws::new(&state().base_url, format!("main/{}", id), {
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
                            playlist_state.0.debounce.set(Utc::now());
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

fn debounce_pass(state: &PlaylistState) -> bool {
    return Utc::now() - state.0.debounce.get() >= Duration::milliseconds(50);
}

pub struct PlaylistPushArg {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub cover_source_url: Option<SourceUrl>,
    pub source_url: SourceUrl,
    pub media_type: PlaylistEntryMediaType,
}

pub fn playlist_extend(
    pc: &mut ProcessingContext,
    playlist_state: &PlaylistState,
    menu_item_id: &String,
    menu_title: &String,
    entries: Vec<(PlaylistIndex, PlaylistPushArg)>,
    restore_pos: &Option<PlaylistRestorePos>,
) {
    *playlist_state.0.ministate_menu_item_id_title.borrow_mut() = Some((menu_item_id.clone(), menu_title.clone()));
    for (entry_index, entry) in entries {
        let setup_media_element = |pc: &mut ProcessingContext, media: &El| {
            media.ref_on("ended", {
                let eg = pc.eg();
                let entry_index = entry_index.clone();
                move |_| eg.event(|pc| {
                    playlist_next(pc, &state().playlist, Some(entry_index.clone()));
                }).unwrap()
            });
            media.ref_on("pause", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    if !debounce_pass(&state().playlist) {
                        return;
                    }
                    state().playlist.0.playing.set(pc, false);
                }).unwrap()
            });
            media.ref_on("play", {
                let eg = pc.eg();
                move |_| eg.event(|pc| {
                    if !debounce_pass(&state().playlist) {
                        return;
                    }
                    state().playlist.0.playing.set(pc, true);
                }).unwrap()
            });
            if let Some(restore_pos) = restore_pos {
                if restore_pos.index == entry_index {
                    media.ref_on("loadedmetadata", {
                        let time = restore_pos.time;
                        move |e| {
                            e.target().unwrap().dyn_into::<HtmlMediaElement>().unwrap().set_current_time(time);
                        }
                    });
                    playlist_state.0.playing_i.set(pc, Some(entry_index.clone()));
                }
            }
        };
        let box_media: Box<dyn PlaylistMedia>;
        match entry.media_type {
            PlaylistEntryMediaType::Audio => {
                let media = el_audio(&entry.source_url.url).attr("controls", "true");
                setup_media_element(pc, &media);
                box_media = Box::new(AudioPlaylistMedia { element: media.clone() });
            },
            PlaylistEntryMediaType::Video => {
                let mut sub_tracks = vec![];
                for lang in window().navigator().languages() {
                    let lang = lang.as_string().unwrap();
                    sub_tracks.push((generated_file_url(&entry.source_url.url, &format!("webvtt_{}", {
                        let lang = if let Some((lang, _)) = lang.split_once("-") {
                            lang
                        } else {
                            &lang
                        };
                        match lang {
                            "en" => "eng",
                            "jp" => "jpn",
                            _ => {
                                log(format!("Unhandled subtitle translation for language {}", lang));
                                continue;
                            },
                        }
                    }), "text/vtt"), lang));
                }
                let media =
                    el_video(
                        &generated_file_url(&entry.source_url.url, "", "video/webm"),
                    ).attr("controls", "true");
                setup_media_element(pc, &media);
                for (i, (url, lang)) in sub_tracks.iter().enumerate() {
                    let track = el("track").attr("kind", "subtitles").attr("src", url).attr("srclang", lang);
                    if i == 0 {
                        track.ref_attr("default", "default");
                    }
                    media.ref_push(track);
                }
                box_media = Box::new(VideoPlaylistMedia { element: media.clone() });
            },
            PlaylistEntryMediaType::Image => {
                let media = el("img").attr("src", &entry.source_url.url).attr("loading", "lazy");
                box_media = Box::new(ImagePlaylistMedia { element: media.clone() });
            },
        }
        playlist_state.0.playlist.borrow_mut().insert(entry_index, Rc::new(PlaylistEntry {
            name: entry.name,
            album: entry.album,
            artist: entry.artist,
            cover_source_url: entry.cover_source_url,
            source_url: entry.source_url,
            media_type: entry.media_type,
            media: box_media,
        }));
    }
}

pub fn playlist_clear(pc: &mut ProcessingContext, state: &PlaylistState) {
    state.0.playing.set(pc, false);
    state.0.playing_i.set(pc, None);
    state.0.playing_max_time.set(pc, None);
    state.0.playlist.borrow_mut().clear();
}

pub fn playlist_toggle_play(pc: &mut ProcessingContext, state: &PlaylistState, i: Option<PlaylistIndex>) {
    if *state.0.playing.borrow() {
        let current_i = state.0.playing_i.get().unwrap();
        let i = i.as_ref().unwrap_or(&current_i);
        if &current_i == i {
            state.0.playing.set(pc, false);
        } else {
            state.0.playing_i.set(pc, Some(i.clone()));
        }
    } else {
        if state.0.playlist.borrow().is_empty() {
            return;
        }
        let i = i.or(state.0.playing_i.get()).unwrap_or(playlist_first_index(state).unwrap());
        state.0.playing_i.set(pc, Some(i));
        state.0.playing.set(pc, true);
    }
}

pub fn playlist_play(pc: &mut ProcessingContext, state: &PlaylistState, i: PlaylistIndex) {
    if state.0.playlist.borrow().is_empty() {
        return;
    }
    state.0.playing_i.set(pc, Some(i));
    state.0.playing.set(pc, true);
}

pub fn playlist_next(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<PlaylistIndex>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if let Some((i, _)) = state.0.playlist.borrow().range((Bound::Excluded(i), Bound::Unbounded)).next() {
        state.0.playing_i.set(pc, Some(i.clone()));
    } else {
        state.0.playing_i.set(pc, None);
        state.0.playing_max_time.set(pc, None);
        state.0.playing.set(pc, false);
        state.0.playing_time.set(pc, 0.);
        if let Some((menu_item_id, title)) = state.0.ministate_menu_item_id_title.borrow().as_ref() {
            record_replace_ministate(&Ministate::View(MinistateView {
                menu_item_id: menu_item_id.clone(),
                title: title.clone(),
                pos: None,
            }));
        }
    }
}

pub fn playlist_previous(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<PlaylistIndex>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if let Some((i, _)) = state.0.playlist.borrow().range((Bound::Unbounded, Bound::Excluded(i))).rev().next() {
        state.0.playing_i.set(pc, Some(i.clone()));
    } else {
        state.0.playing_i.set(pc, None);
        state.0.playing_max_time.set(pc, None);
        state.0.playing.set(pc, false);
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

pub fn playlist_seek(_pc: &mut ProcessingContext, state: &PlaylistState, time: f64) {
    let Some(i) = state.0.playing_i.get() else {
        return;
    };
    let pm = state.0.playlist.borrow().get(&i).cloned().unwrap();
    pm.media.pm_seek(time);
}
