use std::{
    any::Any,
    cell::{
        Cell,
        RefCell,
    },
    collections::HashMap,
    panic,
    rc::Rc,
    str::FromStr,
};
use gloo::{
    console::{
        log,
        warn,
    },
    timers::callback::Interval,
    utils::{
        document,
        window,
    },
};
use js_sys::Function;
use lunk::{
    link,
    EventGraph,
    HistPrim,
    Prim,
    ProcessingContext,
};
use reqwasm::http::Request;
use rooting::{
    el,
    scope_any,
    set_root,
    spawn_rooted,
    El,
};
use rooting_forms::{
    BigString,
    Form,
};
use serde::de::DeserializeOwned;
use shared::{
    model::{
        C2SReq,
        FileHash,
        Node,
        Query,
    },
    unenum,
};
use wasm_bindgen::{
    closure::Closure,
    JsCast,
    JsValue,
    UnwrapThrowExt,
};
use web_sys::{
    HtmlAudioElement,
    HtmlMediaElement,
    MediaMetadata,
    MediaSession,
};
use crate::{
    el_general::log,
    ministate::{
        record_replace_ministate,
        Ministate,
        PlaylistEntryPath,
        PlaylistPos,
    },
};

pub trait PlaylistMedia {
    fn pm_play(&self);
    fn pm_stop(&self);
    fn pm_seek_forward(&self, offset_seconds: f64);
    fn pm_seek_backwards(&self, offset_seconds: f64);
    fn pm_seek_to(&self, time_seconds: f64);
    fn pm_get_time(&self) -> f64;
    fn pm_get_max_time(&self) -> Option<f64>;
    fn pm_get_ministate(&self) -> Ministate;
}

pub struct AudioPlaylistMedia {
    pub element: El,
    pub ministate_id: String,
    pub ministate_title: String,
    pub ministate_path: Option<PlaylistEntryPath>,
}

impl AudioPlaylistMedia {
    fn audio(&self) -> HtmlAudioElement {
        return self.element.raw().dyn_ref::<HtmlAudioElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for AudioPlaylistMedia {
    fn pm_play(&self) {
        let audio = self.audio();
        _ = audio.play().unwrap();
    }

    fn pm_stop(&self) {
        let audio = self.audio();
        audio.pause().unwrap();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(audio.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(audio.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        let audio = self.audio();
        audio.set_current_time(time_seconds);
    }

    fn pm_get_time(&self) -> f64 {
        let audio = self.audio();
        return audio.current_time();
    }

    fn pm_get_max_time(&self) -> Option<f64> {
        let audio = self.audio();
        let out = audio.duration();
        if !out.is_finite() {
            return None;
        } else {
            return Some(out);
        }
    }

    fn pm_get_ministate(&self) -> Ministate {
        return Ministate::View {
            id: self.ministate_id.clone(),
            title: self.ministate_title.clone(),
            pos: match self.ministate_path.as_ref() {
                Some(path) => Some(PlaylistPos {
                    entry_path: path.clone(),
                    time: self.pm_get_time(),
                }),
                None => None,
            },
        };
    }
}

pub struct VideoPlaylistMedia {
    pub element: El,
    pub ministate_id: String,
    pub ministate_title: String,
    pub ministate_path: Option<PlaylistEntryPath>,
}

impl VideoPlaylistMedia {
    fn media(&self) -> HtmlMediaElement {
        return self.element.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for VideoPlaylistMedia {
    fn pm_play(&self) {
        let s = self.media();
        match s.request_fullscreen() {
            Err(e) => {
                warn!("Failed to fullscreen video: {}", e);
            },
            _ => { },
        }
        _ = s.play().unwrap();
    }

    fn pm_stop(&self) {
        let s = self.media();
        s.pause().unwrap();
        document().exit_fullscreen();
    }

    fn pm_seek_forward(&self, offset_seconds: f64) {
        let s = self.media();
        s.set_current_time(s.current_time() + offset_seconds);
    }

    fn pm_seek_backwards(&self, offset_seconds: f64) {
        let s = self.media();
        s.set_current_time(s.current_time() - offset_seconds);
    }

    fn pm_seek_to(&self, time_seconds: f64) {
        let s = self.media();
        s.set_current_time(time_seconds);
    }

    fn pm_get_time(&self) -> f64 {
        let s = self.media();
        return s.current_time();
    }

    fn pm_get_max_time(&self) -> Option<f64> {
        let s = self.media();
        let out = s.duration();
        if !out.is_finite() {
            return None;
        } else {
            return Some(out);
        }
    }

    fn pm_get_ministate(&self) -> Ministate {
        return Ministate::View {
            id: self.ministate_id.clone(),
            title: self.ministate_title.clone(),
            pos: match self.ministate_path.as_ref() {
                Some(path) => Some(PlaylistPos {
                    entry_path: path.clone(),
                    time: self.pm_get_time(),
                }),
                None => None,
            },
        };
    }
}

pub struct PlaylistEntry {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub thumbnail: Option<String>,
    pub media: Box<dyn PlaylistMedia>,
}

pub struct PlaylistState_ {
    playlist: RefCell<Vec<Rc<PlaylistEntry>>>,
    pub playing: Prim<bool>,
    // Must be Some if playing, otherwise may be Some.
    pub playing_i: HistPrim<Option<usize>>,
    pub playing_time: Prim<f64>,
    pub playing_max_time: Prim<Option<f64>>,
}

#[derive(Clone)]
pub struct PlaylistState(pub Rc<PlaylistState_>);

pub fn state_new(pc: &mut ProcessingContext) -> (PlaylistState, rooting::ScopeValue) {
    let state = PlaylistState(Rc::new(PlaylistState_ {
        playlist: RefCell::new(vec![]),
        playing: Prim::new(pc, false),
        playing_i: HistPrim::new(pc, None),
        playing_time: Prim::new(pc, 0.),
        playing_max_time: Prim::new(pc, None),
    }));
    let media_session = window().navigator().media_session();

    // # Media control
    fn media_fn(pc: &mut ProcessingContext, f: impl 'static + Fn(&mut ProcessingContext, JsValue) -> ()) -> Function {
        let eg = pc.eg();
        let fn1 = Closure::<dyn Fn(JsValue) -> ()>::wrap(Box::new(move |args| eg.event(|pc| f(pc, args))));
        let fn2: Function = fn1.as_ref().unchecked_ref::<Function>().to_owned();
        fn1.forget();
        return fn2;
    }

    media_session.set_action_handler(web_sys::MediaSessionAction::Play, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, _args| {
            playlist_play(pc, &state);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Pause, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, _args| {
            playlist_pause(pc, &state);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Stop, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, _args| {
            state.0.playing.set(pc, false);
            state.0.playing_i.set(pc, None);
            state.0.playing_max_time.set(pc, None);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Nexttrack, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, _args| {
            playlist_next(pc, &state, state.0.playing_i.get());
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Previoustrack, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, _args| {
            playlist_previous(pc, &state, state.0.playing_i.get());
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekforward, Some(&media_fn(pc, {
        let state = state.clone();
        move |_pc, args| {
            let Some(i) = state.0.playing_i.get() else {
                return;
            };
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let pm = state.0.playlist.borrow().get(i).cloned().unwrap();
            pm.media.pm_seek_forward(offset);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekbackward, Some(&media_fn(pc, {
        let state = state.clone();
        move |_pc, args| {
            let Some(i) = state.0.playing_i.get() else {
                return;
            };
            let offset = js_sys::Reflect::get(&args, &JsValue::from("seekOffset")).unwrap().as_f64().unwrap_or(5.);
            let pm = state.0.playlist.borrow().get(i).cloned().unwrap();
            pm.media.pm_seek_backwards(offset);
        }
    })));
    media_session.set_action_handler(web_sys::MediaSessionAction::Seekto, Some(&media_fn(pc, {
        let state = state.clone();
        move |pc, args| {
            let time = js_sys::Reflect::get(&args, &JsValue::from("seekTime")).unwrap().as_f64().unwrap();
            playlist_seek(pc, &state, time);
        }
    })));
    return (state.clone(), scope_any((
        //. .
        link!(
            //. .
            (_pc = pc),
            (playing = state.0.playing.clone(), playing_i = state.0.playing_i.clone()),
            (),
            (state = state.clone(), media_session = media_session) {
                if !*playing.borrow() {
                    // Stop previous
                    if let Some(i) = playing_i.get_old() {
                        if let Some(e) = state.0.playlist.borrow().get(i).cloned() {
                            e.media.pm_stop();
                        }
                    }
                } else {
                    // Stop previous if it changed
                    if let Some(i) = playing_i.get_old() {
                        if Some(i) != playing_i.get() {
                            let e = state.0.playlist.borrow().get(i).cloned().unwrap();
                            e.media.pm_stop();
                            e.media.pm_seek_to(0.);
                        }
                    }

                    // Start next/current
                    let i = match playing_i.get() {
                        Some(i) => i,
                        None => {
                            0
                        },
                    };
                    let e = state.0.playlist.borrow().get(i).cloned().unwrap();
                    e.media.pm_play();
                }
                match state.0.playing_i.get() {
                    Some(i) => {
                        let e = state.0.playlist.borrow().get(i).cloned().unwrap();
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
                            if let Some(thumbnail) = &e.thumbnail {
                                let arr = js_sys::Array::new();
                                let e = js_sys::Object::new();
                                js_sys::Reflect::set(&e, &JsValue::from("src"), &JsValue::from(thumbnail)).unwrap();
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
            }
        ),
        Interval::new(1000, {
            let state = state.clone();
            let eg = pc.eg();
            let last_state = Cell::new(None);
            move || {
                let Some(playing_i) =&* state.0.playing_i.borrow() else {
                    return;
                };
                let time;
                let max_time;
                let ministate;
                {
                    let playlist = state.0.playlist.borrow();
                    let entry = playlist.get(*playing_i).unwrap();
                    time = entry.media.pm_get_time();
                    max_time = entry.media.pm_get_max_time();
                    ministate = entry.media.pm_get_ministate();
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
                record_replace_ministate(&ministate);
            }
        }),
    )));
}

pub fn playlist_len(state: &PlaylistState) -> usize {
    return state.0.playlist.borrow().len();
}

pub fn playlist_push(state: &PlaylistState, e: Rc<PlaylistEntry>) {
    state.0.playlist.borrow_mut().push(e);
}

pub fn playlist_clear(pc: &mut ProcessingContext, state: &PlaylistState) {
    state.0.playing.set(pc, false);
    state.0.playing_i.set(pc, None);
    state.0.playing_max_time.set(pc, None);
    state.0.playlist.borrow_mut().clear();
}

pub fn playlist_toggle_play(pc: &mut ProcessingContext, state: &PlaylistState, i: Option<usize>) {
    if *state.0.playing.borrow() {
        let current_i = state.0.playing_i.get().unwrap();
        let i = i.unwrap_or(current_i);
        if current_i == i {
            state.0.playing.set(pc, false);
        } else {
            state.0.playing_i.set(pc, Some(i));
        }
    } else {
        if state.0.playlist.borrow().is_empty() {
            return;
        }
        let i = i.or(state.0.playing_i.get()).unwrap_or(0);
        state.0.playing_i.set(pc, Some(i));
        state.0.playing.set(pc, true);
    }
}

pub fn playlist_next(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<usize>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if i + 1 < state.0.playlist.borrow().len() {
        state.0.playing_i.set(pc, Some(i + 1));
    } else {
        state.0.playing_i.set(pc, None);
        state.0.playing_max_time.set(pc, None);
        state.0.playing.set(pc, false);
    }
}

pub fn playlist_previous(pc: &mut ProcessingContext, state: &PlaylistState, basis: Option<usize>) {
    let Some(i) = basis.or(state.0.playing_i.get()) else {
        return;
    };
    if i > 0 {
        state.0.playing_i.set(pc, Some(i - 1));
    } else {
        state.0.playing_i.set(pc, None);
        state.0.playing_max_time.set(pc, None);
        state.0.playing.set(pc, false);
    }
}

pub fn playlist_pause(pc: &mut ProcessingContext, state: &PlaylistState) {
    state.0.playing.set(pc, false);
}

pub fn playlist_play(pc: &mut ProcessingContext, state: &PlaylistState) {
    if state.0.playlist.borrow().is_empty() {
        return;
    }
    if *state.0.playing.borrow() {
        return;
    }
    if state.0.playing_i.borrow().is_none() {
        state.0.playing_i.set(pc, Some(0));
    }
    state.0.playing.set(pc, true);
}

pub fn playlist_seek(_pc: &mut ProcessingContext, state: &PlaylistState, time: f64) {
    let Some(i) = state.0.playing_i.get() else {
        return;
    };
    let pm = state.0.playlist.borrow().get(i).cloned().unwrap();
    pm.media.pm_seek_to(time);
}
