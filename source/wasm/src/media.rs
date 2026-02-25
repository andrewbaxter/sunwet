//! TODO - instead of playing media, there should be one "player" that has all the
//! methods. It should keep a `bg` element, where each action clears and sets the
//! bg action if asynchronous, so the methods return immediately but are
//! synchronized.
//!
//! This should allow nixing the play_bg. Also the hash could be passed in instead
//! of all the urls.
use {
    crate::js::{
        ElExt,
        Env,
        Log,
        LogJsErr,
        MyIntersectionObserver,
        async_event,
        el_async,
        style_export,
    },
    flowcontrol::ta_return,
    futures::{
        FutureExt,
        StreamExt,
    },
    gloo::{
        events::{
            EventListener,
            EventListenerOptions,
        },
        timers::{
            callback::Timeout,
            future::{
                TimeoutFuture,
                sleep,
            },
        },
        utils::{
            document,
            window,
        },
    },
    lunk::{
        EventGraph,
        Prim,
        ProcessingContext,
        link,
    },
    reqwasm::http::Request,
    rooting::{
        El,
        ScopeValue,
        el,
        spawn_rooted,
    },
    shared::interface::{
        derived::ComicManifest,
        wire::GEN_FILENAME_COMICMANIFEST,
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::{
            HashMap,
            HashSet,
        },
        future::Future,
        pin::Pin,
        rc::Rc,
        str::FromStr,
        time::Duration,
    },
    tokio::sync::watch,
    tokio_stream::wrappers::WatchStream,
    wasm_bindgen::{
        JsCast,
        convert::{
            FromWasmAbi,
            IntoWasmAbi,
        },
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        Document,
        Element,
        HtmlElement,
        HtmlIFrameElement,
        HtmlMediaElement,
        KeyboardEvent,
        MouseEvent,
        WheelEvent,
    },
};

pub trait PlaylistMedia {
    /// Audio - no display, video/image show fs overlay based on element
    fn pm_display(&self) -> bool;

    /// Transition to auto-advancing
    fn pm_play(&self, log: &Rc<dyn Log>);
    fn pm_stop(&self);
    fn pm_get_time(&self) -> f64;
    fn pm_get_max_time(&self) -> Option<f64>;
    fn pm_format_time(&self, time: f64) -> String;
    fn pm_seek(&self, pc: &mut ProcessingContext, time: f64);
    fn pm_preload(&self, log: &Rc<dyn Log>, env: &Env);
    fn pm_unpreload(&self, log: &Rc<dyn Log>);
    fn pm_el(&self, log: &Rc<dyn Log>, pc: &mut ProcessingContext) -> El;
    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct PlaylistMediaAudioVideo {
    pub video: bool,
    pub media_el: HtmlMediaElement,
    pub el: El,
    pub src: String,
    pub sub_src: HashMap<String, String>,
    pub play_bg: Rc<RefCell<Option<ScopeValue>>>,
    pub time: Cell<f64>,
}

impl PlaylistMediaAudioVideo {
    pub fn new_audio(el: El, src: String, time: f64) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            video: false,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            src: src,
            sub_src: Default::default(),
            play_bg: Default::default(),
            time: Cell::new(time),
        };
    }

    pub fn new_video(el: El, src: String, sub_src: HashMap<String, String>, time: f64) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            video: true,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            src: src,
            sub_src: sub_src,
            play_bg: Default::default(),
            time: Cell::new(time),
        };
    }
}

impl PlaylistMedia for PlaylistMediaAudioVideo {
    fn pm_display(&self) -> bool {
        return self.video;
    }

    fn pm_el(&self, _log: &Rc<dyn Log>, _pc: &mut ProcessingContext) -> El {
        return self.el.clone();
    }

    fn pm_play(&self, log: &Rc<dyn Log>) {
        fn do_play(log: &Rc<dyn Log>, bg: Rc<RefCell<Option<ScopeValue>>>, media_el: HtmlMediaElement) {
            let f = match media_el.play() {
                Ok(f) => f,
                Err(e) => {
                    log.log_js("Error playing video", &e);
                    return;
                },
            };
            let f1 = {
                let bg = bg.clone();
                let log = log.clone();
                async move {
                    match JsFuture::from(f).await {
                        Ok(_) => { },
                        Err(e) => {
                            log.log_js("Error playing media, retrying in 1s", &e);
                            let src = media_el.src();
                            media_el.set_src("");
                            TimeoutFuture::new(1000).await;
                            media_el.set_src(&src);
                            do_play(&log, bg, media_el);
                        },
                    };
                }
            };
            *bg.borrow_mut() = Some(spawn_rooted(f1));
        }

        do_play(log, self.play_bg.clone(), self.media_el.clone());
    }

    fn pm_stop(&self) {
        self.media_el.pause().unwrap();
        let mut play_bg = self.play_bg.borrow_mut();
        if play_bg.is_some() {
            self.time.set(self.media_el.current_time());
        }
        *play_bg = None;
    }

    fn pm_get_max_time(&self) -> Option<f64> {
        let out = self.media_el.duration();
        if !out.is_finite() {
            return None;
        } else {
            return Some(out);
        }
    }

    fn pm_get_time(&self) -> f64 {
        let out = self.media_el.current_time();
        self.time.set(out);
        return out;
    }

    fn pm_format_time(&self, time: f64) -> String {
        let time = time as u64;
        let seconds = time % 60;
        let time = time / 60;
        let minutes = time % 60;
        let time = time / 60;
        let hours = time % 24;
        let days = time / 24;
        if days > 0 {
            return format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds);
        } else if hours > 0 {
            return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        } else {
            return format!("{:02}:{:02}", minutes, seconds);
        }
    }

    fn pm_seek(&self, _pc: &mut ProcessingContext, time: f64) {
        self.media_el.set_current_time(time);
        self.time.set(time);
    }

    fn pm_preload(&self, log: &Rc<dyn Log>, env: &Env) {
        self.media_el.set_attribute("preload", "auto").log(log, "Error setting preload attribute");
        if self.src != self.media_el.current_src() {
            if self.video {
                self.media_el.set_inner_html("");
                for (i, lang) in env.languages.iter().enumerate() {
                    let Some(sub_src) = self.sub_src.get(lang) else {
                        continue;
                    };
                    let track = el("track").attr("kind", "subtitles").attr("src", &sub_src).attr("srclang", &lang);
                    if i == 0 {
                        track.ref_attr("default", "default");
                    }
                    self.media_el.append_child(&track.raw()).log(log, "Error adding track to video element");
                }
            }
            self.media_el.set_src(&self.src);

            // iOS doesn't load unless load called. Load resets currentTime to 0 on chrome
            // (desktop/android). Avoid calling load again unless the url changes (to avoid
            // time reset).
            self.media_el.load();
        }
    }

    fn pm_unpreload(&self, log: &Rc<dyn Log>) {
        self.media_el.set_attribute("preload", "metadata").log(log, "Error reducing preload attribute");
    }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.media_el.clone();
        return async move {
            // 1 = `HAVE_METADATA`
            if m.ready_state() < 1 {
                async_event(&m, "loadedmetadata").await;
            }
        }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let m = self.media_el.clone();
        return async move {
            // 4 = `HAVE_ENOUGH_DATA`
            if m.ready_state() < 4 {
                async_event(&m, "canplaythrough").await;
            }
        }.boxed_local();
    }
}

pub struct PlaylistMediaImage {
    pub element: El,
    pub src: String,
}

impl PlaylistMediaImage { }

impl PlaylistMedia for PlaylistMediaImage {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, _log: &Rc<dyn Log>, _pc: &mut ProcessingContext) -> El {
        return self.element.clone();
    }

    fn pm_play(&self, _log: &Rc<dyn Log>) { }

    fn pm_stop(&self) { }

    fn pm_get_max_time(&self) -> Option<f64> {
        return None;
    }

    fn pm_get_time(&self) -> f64 {
        return 0.;
    }

    fn pm_format_time(&self, time: f64) -> String {
        return format!("{}", time as usize);
    }

    fn pm_seek(&self, _pc: &mut ProcessingContext, _time: f64) { }

    fn pm_preload(&self, _log: &Rc<dyn Log>, _env: &Env) {
        self.element.ref_attr("loading", "eager");
        self.element.ref_attr("src", &self.src);
    }

    fn pm_unpreload(&self, _log: &Rc<dyn Log>) {
        self.element.ref_attr("loading", "lazy");
    }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

const ATTR_INDEX: &str = "data-index";

pub struct MediaComicManifestPage {
    pub width: u32,
    pub height: u32,
    pub url: String,
}

pub struct MediaComicManifest {
    pub rtl: bool,
    pub pages: Vec<MediaComicManifestPage>,
}

type PlaylistMediaComicReqManifestFn =
    Rc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<MediaComicManifest, String>>>>>;

pub struct PlaylistMediaComic {
    pub length: Rc<Cell<Option<usize>>>,
    pub seekable: watch::Sender<bool>,
    pub at: Prim<usize>,
    pub req_manifest: PlaylistMediaComicReqManifestFn,
}

impl PlaylistMediaComic {
    pub fn new(req_manifest: PlaylistMediaComicReqManifestFn, restore_index: usize) -> Self {
        return Self {
            seekable: watch::channel(false).0,
            length: Rc::new(Cell::new(None)),
            at: Prim::new(restore_index),
            req_manifest: req_manifest,
        };
    }
}

pub fn comic_req_fn_online(log: &Rc<dyn Log>, base_url: String) -> PlaylistMediaComicReqManifestFn {
    let log = log.clone();
    return Rc::new(move || {
        let log = log.clone();
        let dir_url = base_url.clone();
        async move {
            loop {
                match async {
                    ta_return!(MediaComicManifest, String);
                    let r =
                        Request::get(&format!("{}/{}", dir_url, GEN_FILENAME_COMICMANIFEST))
                            .send()
                            .await
                            .map_err(|e| format!("Error requesting comic manifest: {}", e))?
                            .binary()
                            .await
                            .map_err(|e| format!("Error reading comic manifest response: {}", e))?;
                    let raw_manifest =
                        serde_json::from_slice::<ComicManifest>(
                            &r,
                        ).map_err(|e| format!("Error reading comic manifest: {}", e))?;
                    return Ok(MediaComicManifest {
                        rtl: raw_manifest.rtl,
                        pages: raw_manifest.pages.into_iter().map(|x| MediaComicManifestPage {
                            width: x.width,
                            height: x.height,
                            url: format!("{}/{}", dir_url, x.path),
                        }).collect(),
                    });
                }.await {
                    Ok(r) => return Ok(r),
                    Err(e) => {
                        log.log(&format!("Request failed, retrying: {}", e));
                        sleep(Duration::from_secs(1)).await;
                    },
                }
            }
        }
    }.boxed_local());
}

impl PlaylistMedia for PlaylistMediaComic {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, _log: &Rc<dyn Log>, pc: &mut ProcessingContext) -> El {
        _ = self.seekable.send(false);
        let req_manifest = self.req_manifest.clone();
        let at = self.at.clone();
        let length = self.length.clone();
        let eg = pc.eg();
        let seekable = self.seekable.clone();

        // Super outer container, pads outer when height is reduced
        let root = style_export::cont_media_comic_outer(style_export::ContMediaComicOuterArgs { children: vec![] }).root;
        root.ref_push(el_async(async move {
            ta_return!(Vec < El >, String);
            let manifest = req_manifest().await?;
            let rtl = manifest.rtl;

            // Populate pages, gather page info, organize
            struct PageLookupEntry {
                page_in_group: usize,
                group_in_media: usize,
            }

            struct BuildState {
                wip_group: Vec<El>,
                groups: Vec<Vec<El>>,
                page_lookup: HashMap<usize, PageLookupEntry>,
            }

            impl BuildState {
                fn build_flush(&mut self) {
                    if self.wip_group.is_empty() {
                        return;
                    }
                    self.groups.push(self.wip_group.split_off(0));
                }
            }

            let mut build_state = BuildState {
                wip_group: Default::default(),
                groups: Default::default(),
                page_lookup: Default::default(),
            };
            let mut page_children = vec![];
            let mut min_aspect = 1.;
            for (i, page) in manifest.pages.iter().enumerate() {
                let page_el = style_export::leaf_media_comic_page(style_export::LeafMediaComicPageArgs {
                    src: page.url.clone(),
                    aspect_x: page.width.to_string(),
                    aspect_y: page.height.to_string(),
                }).root;
                page_el.ref_attr(ATTR_INDEX, &i.to_string());
                let vert_aspect = page.width as f64 / page.height as f64;
                if vert_aspect < min_aspect {
                    min_aspect = vert_aspect;
                }

                // Pre-group pad
                if i == 0 {
                    page_children.push(style_export::leaf_media_comic_end_pad().root);
                } else if build_state.wip_group.is_empty() {
                    page_children.push(style_export::leaf_media_comic_mid_pad().root);
                }

                // File page
                if page.width > page.height {
                    build_state.build_flush();
                }
                build_state.page_lookup.insert(i, PageLookupEntry {
                    page_in_group: build_state.wip_group.len(),
                    group_in_media: build_state.groups.len(),
                });
                build_state.wip_group.push(page_el.clone());
                if page.width > page.height || i == 0 || build_state.wip_group.len() == 2 {
                    build_state.build_flush();
                }
                page_children.push(page_el);

                // Final page post-pad
                if i == manifest.pages.len() - 1 {
                    page_children.push(style_export::leaf_media_comic_end_pad().root);
                }
            }
            build_state.build_flush();
            let res = style_export::cont_media_comic_inner(style_export::ContMediaComicInnerArgs {
                min_aspect_x: min_aspect.to_string(),
                min_aspect_y: "1".to_string(),
                children: page_children,
                rtl: rtl,
            });

            struct State {
                rtl: bool,
                at: Prim<usize>,
                inner: El,
                groups: Vec<Vec<El>>,
                page_lookup: HashMap<usize, PageLookupEntry>,
                fix_scroll: RefCell<Option<ScopeValue>>,
            }

            impl State {
                fn view_width(&self) -> f64 {
                    self.inner.html().get_bounding_client_rect().width()
                }

                fn set_scroll_center(&self, want_center: f64) {
                    self.inner.html().set_scroll_left((want_center - self.view_width() / 2.) as i32);
                }

                fn get_scroll_center(&self) -> f64 {
                    return self.inner.html().scroll_left() as f64 + self.view_width() / 2.;
                }

                fn calc_group_width(&self, group: &Vec<El>) -> f64 {
                    return group.iter().map(|x| x.html().get_bounding_client_rect().width()).sum();
                }

                fn want_group_movement(&self, group_width: f64) -> bool {
                    return group_width <= self.view_width();
                }

                fn calc_want_center(&self, index: usize) -> f64 {
                    let index = index.min(self.page_lookup.len());
                    let entry = self.page_lookup.get(&index).unwrap();
                    let group = &self.groups[entry.group_in_media];
                    let group_width = self.calc_group_width(group);
                    if self.want_group_movement(group_width) {
                        return if self.rtl {
                            group.last().unwrap()
                        } else {
                            group.first().unwrap()
                        }.html().offset_left() as f64 + group_width / 2.;
                    } else {
                        let e = &group[entry.page_in_group].html();
                        return e.offset_left() as f64 + e.get_bounding_client_rect().width() / 2.;
                    }
                }

                fn seek(&self, pc: &mut ProcessingContext, new_index: usize) {
                    self.at.set(pc, new_index);
                }

                fn seek_next(&self, pc: &mut ProcessingContext) {
                    let entry = self.page_lookup.get(&*self.at.borrow()).unwrap();
                    let group = &self.groups[entry.group_in_media];
                    let new_index;
                    if self.want_group_movement(self.calc_group_width(group)) {
                        new_index = *self.at.borrow() + (group.len() - entry.page_in_group);
                    } else {
                        new_index = *self.at.borrow() + 1;
                    }
                    self.seek(pc, new_index);
                }

                fn seek_prev(&self, pc: &mut ProcessingContext) {
                    let entry = self.page_lookup.get(&*self.at.borrow()).unwrap();
                    let group = &self.groups[entry.group_in_media];
                    let new_index;
                    if self.want_group_movement(self.calc_group_width(group)) {
                        new_index = *self.at.borrow() - 1 - entry.page_in_group;
                    } else {
                        new_index = *self.at.borrow() - 1;
                    }
                    self.seek(pc, new_index);
                }
            }

            let state = Rc::new(State {
                rtl: rtl,
                at: at.clone(),
                inner: res.cont_scroll,
                groups: build_state.groups,
                page_lookup: build_state.page_lookup,
                fix_scroll: RefCell::new(None),
            });

            // Wait for browser ready
            length.set(Some(manifest.pages.len()));
            res.root.ref_own({
                let outer = res.root.weak();
                move |_| spawn_rooted(async move {
                    loop {
                        let want_center = state.calc_want_center(*at.borrow());
                        state.set_scroll_center(want_center);
                        if (state.get_scroll_center() - want_center).abs() < 3. {
                            break;
                        }
                        TimeoutFuture::new(100).await;
                    }

                    // Finish hooking things up
                    eg.event(|pc| {
                        let Some(outer) = outer.upgrade() else {
                            return;
                        };
                        let visible = Rc::new(RefCell::new(HashSet::new()));
                        let io = MyIntersectionObserver::new(0., {
                            let visible = visible.clone();
                            move |entries| {
                                for entry in entries {
                                    let index =
                                        usize::from_str(
                                            &entry.target().get_attribute(ATTR_INDEX).unwrap(),
                                        ).unwrap();
                                    if entry.is_intersecting() {
                                        visible.borrow_mut().insert(index);
                                    } else {
                                        visible.borrow_mut().remove(&index);
                                    }
                                }
                            }
                        });
                        for group in &state.groups {
                            for page in group {
                                io.observe(&page.raw());
                            }
                        }
                        let scroll_at = Prim::new(*at.borrow());
                        state.inner.ref_on_resize({
                            let state = Rc::downgrade(&state);
                            let scroll_at = scroll_at.clone();
                            move |_, _, _| {
                                let Some(state1) = state.upgrade() else {
                                    return;
                                };
                                *state1.fix_scroll.borrow_mut() = Some(spawn_rooted({
                                    let state = state.clone();
                                    let scroll_at = scroll_at.clone();
                                    async move {
                                        loop {
                                            {
                                                let Some(state) = state.upgrade() else {
                                                    return;
                                                };
                                                let want_center = state.calc_want_center(*scroll_at.borrow());
                                                state.set_scroll_center(want_center);
                                                if (state.get_scroll_center() - want_center).abs() < 3. {
                                                    break;
                                                }
                                            }
                                            TimeoutFuture::new(100).await;
                                        }
                                        let Some(state) = state.upgrade() else {
                                            return;
                                        };
                                        *state.fix_scroll.borrow_mut() = None;
                                    }
                                }));
                            }
                        });
                        state.inner.ref_on("scroll", {
                            let visible = visible.clone();
                            let state = Rc::downgrade(&state);
                            let bg = Cell::new(None);
                            let eg = pc.eg();
                            let scroll_at = scroll_at.clone();
                            move |_| bg.set(Some(Timeout::new(300, {
                                let state = state.clone();
                                let eg = eg.clone();
                                let visible = visible.clone();
                                let scroll_at = scroll_at.clone();
                                move || {
                                    let Some(state) = state.upgrade() else {
                                        return;
                                    };
                                    if state.fix_scroll.borrow().is_some() {
                                        return;
                                    }
                                    let visible = visible.borrow().clone();
                                    let view_center = state.get_scroll_center();
                                    for index in visible {
                                        let entry = state.page_lookup.get(&index).unwrap();
                                        let group = &state.groups[entry.group_in_media];
                                        let e = group[entry.page_in_group].html();
                                        let e_left = e.offset_left() as f64;
                                        if view_center >= e_left &&
                                            view_center <= e_left + e.get_bounding_client_rect().width() {
                                            eg.event(|pc| {
                                                scroll_at.set(pc, index);
                                            }).unwrap();
                                            break;
                                        }
                                    }
                                }
                            })))
                        });
                        outer.ref_own(|_| (
                            //. .
                            io,
                            link!(
                                (pc = pc),
                                (external_at = state.at.clone()),
                                (scroll_at = scroll_at.clone()),
                                (state = state.clone()),
                                {
                                    let seek = *external_at.borrow();
                                    scroll_at.set(pc, seek);
                                    state.set_scroll_center(state.calc_want_center(seek));
                                }
                            ),
                            link!((pc = pc), (scroll_at = scroll_at.clone()), (external_at = state.at.clone()), (), {
                                external_at.set(pc, *scroll_at.borrow());
                            }),
                        ));
                        outer.ref_own(|_| EventListener::new_with_options(&window(), "keydown", EventListenerOptions {
                            passive: false,
                            ..Default::default()
                        }, {
                            let eg = eg.clone();
                            let state = state.clone();
                            move |ev| eg.event(|pc| {
                                let ev = ev.dyn_ref::<KeyboardEvent>().unwrap();
                                match ev.key().as_str() {
                                    "ArrowLeft" => {
                                        if rtl {
                                            state.seek_next(pc);
                                        } else {
                                            state.seek_prev(pc);
                                        }
                                    },
                                    "ArrowRight" => {
                                        if rtl {
                                            state.seek_prev(pc);
                                        } else {
                                            state.seek_next(pc);
                                        }
                                    },
                                    " " | "Enter" => {
                                        state.seek_next(pc);
                                    },
                                    "Backspace" => {
                                        state.seek_prev(pc);
                                    },
                                    _ => {
                                        return;
                                    },
                                }
                                ev.stop_propagation();
                                ev.prevent_default();
                            }).unwrap()
                        }));
                        outer.ref_on("click", {
                            let eg = eg.clone();
                            let state = state.clone();
                            move |ev| eg.event(|pc| {
                                let ev = ev.dyn_ref::<MouseEvent>().unwrap();
                                let outer_box = state.inner.html().get_bounding_client_rect();
                                let percent = (ev.client_x() as f64 - outer_box.x()) / outer_box.width();
                                if percent < 0.5 {
                                    if rtl {
                                        state.seek_next(pc);
                                    } else {
                                        state.seek_prev(pc);
                                    }
                                } else {
                                    if rtl {
                                        state.seek_prev(pc);
                                    } else {
                                        state.seek_next(pc);
                                    }
                                }
                            }).unwrap()
                        });
                        outer.ref_on("wheel", {
                            let eg = eg.clone();
                            let state = state.clone();
                            move |ev| eg.event(|pc| {
                                let ev = ev.dyn_ref::<WheelEvent>().unwrap();
                                if ev.delta_x() != 0. || ev.delta_z() != 0. {
                                    return;
                                }
                                if ev.delta_y() >= 0. {
                                    state.seek_next(pc);
                                } else {
                                    state.seek_prev(pc);
                                }
                            }).unwrap()
                        });
                        _ = seekable.send(true);
                    }).unwrap();
                })
            });
            return Ok(vec![res.root]);
        }));
        return root;
    }

    fn pm_play(&self, _log: &Rc<dyn Log>) { }

    fn pm_stop(&self) { }

    fn pm_get_max_time(&self) -> Option<f64> {
        return self.length.get().map(|x| x as f64);
    }

    fn pm_get_time(&self) -> f64 {
        return *self.at.borrow() as f64;
    }

    fn pm_format_time(&self, time: f64) -> String {
        return format!("p{}", time as usize);
    }

    fn pm_seek(&self, pc: &mut ProcessingContext, time: f64) {
        self.at.set(pc, time as usize);
    }

    fn pm_preload(&self, _log: &Rc<dyn Log>, _env: &Env) { }

    fn pm_unpreload(&self, _log: &Rc<dyn Log>) { }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let mut seekable = WatchStream::new(self.seekable.subscribe());
        return async move {
            while let Some(false) = seekable.next().await { }
        }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

pub struct PlaylistMediaBook {
    pub length: Rc<Cell<Option<usize>>>,
    pub seekable: watch::Sender<bool>,
    pub at: Prim<usize>,
    pub url: String,
}

impl PlaylistMediaBook {
    pub fn new(url: &str, restore_index: usize) -> Self {
        return PlaylistMediaBook {
            length: Rc::new(Cell::new(None)),
            at: Prim::new(restore_index),
            seekable: watch::channel(false).0,
            url: url.to_string(),
        };
    }
}

fn setup_book_idoc(
    log: &Rc<dyn Log>,
    eg: EventGraph,
    iframe: El,
    idoc: Document,
    external_at: Prim<usize>,
    scroll_at: Prim<usize>,
    length: Rc<Cell<Option<usize>>>,
    seekable: watch::Sender<bool>,
) {
    if let Some(body) = idoc.body() {
        body
            .style()
            .set_property("font-size", &style_export::book_base_font_size().value)
            .log(log, "Error setting base font size in iframe");
    }

    // Wait for stuff to appear, not sure why this isn't handled by
    // DOMContentLoaded... but I guess in some cases there might be js doing stuff too
    let html_children0 = idoc.query_selector_all("h1,h2,h3,h4,h5,h6,p,img").unwrap();
    if html_children0.length() == 0 {
        log.log("Book iframe body has no typical document elements (h*,p,img), can't integrate");
        return;
    }
    let mut html_children = vec![];

    fn to_html_element(n: impl IntoWasmAbi<Abi = u32>) -> HtmlElement {
        // https://github.com/rustwasm/wasm-bindgen/issues/4521
        // https://stackoverflow.com/questions/59156177/type-safe-way-to-check-instanceof-while-working-with-iframes
        // Hack
        return unsafe {
            HtmlElement::from_abi(n.into_abi())
        };
    }

    for i in 0 .. html_children0.length() {
        let child = to_html_element(html_children0.item(i).unwrap());
        child.set_attribute(ATTR_INDEX, &format!("{}", i)).log(log, "Error setting book element index");
        html_children.push(child);
    }
    length.set(Some(html_children.len()));
    iframe.ref_own(|iframe| spawn_rooted({
        let iframe = iframe.weak();
        async move {
            // Prep
            let get_child_coord = {
                let ibody = idoc.body().unwrap();
                move |c: &Element| -> f64 {
                    return c.get_bounding_client_rect().top() - ibody.get_bounding_client_rect().top();
                }
            };
            let scroll_root = idoc.body().unwrap().parent_element().unwrap();
            let get_scroll_coord = {
                let m = scroll_root.clone();
                move || {
                    return m.scroll_top() as f64;
                }
            };
            let set_scroll_coord = {
                let m = scroll_root.clone();
                move |v: f64| {
                    m.set_scroll_top(v as i32);
                }
            };

            // Do initial scroll restore - don't set up observers yet to avoid
            // feedback/unnecessary noise
            let restore_e = &html_children[*external_at.borrow()];
            loop {
                // 5 to avoid rounding errors
                let want_scroll = get_child_coord(&restore_e) + 5.;
                set_scroll_coord(want_scroll);
                if (get_scroll_coord() - want_scroll).abs() < 3. {
                    break;
                }
                TimeoutFuture::new(100).await;
            }

            // Start observing. The current element is the element straddling the top of the
            // view. It will be shifted to exactly the top of the view when seeking/restoring.
            //
            // Listen for elements coming from off screen (newly visible)
            let io_far = MyIntersectionObserver::new(0., {
                let scroll_at = scroll_at.clone();
                let eg = eg.clone();
                let get_child_coord = get_child_coord.clone();
                let get_scroll_coord = get_scroll_coord.clone();
                move |entries| eg.event(|pc| {
                    // Find the lowest non->intersecting entry off the top.
                    let Some((_, e)) = entries.into_iter().filter_map(|entry| {
                        if !entry.is_intersecting() {
                            return None;
                        }
                        let e = to_html_element(entry.target());
                        let coord = get_child_coord(&e);
                        if coord > get_scroll_coord() {
                            // Not on top-side
                            return None;
                        }
                        return Some((coord, e));
                    }).min_by(|a, b| f64::total_cmp(&a.0, &b.0)) else {
                        return;
                    };
                    scroll_at.set(pc, usize::from_str(&e.get_attribute(ATTR_INDEX).unwrap()).unwrap());
                }).unwrap()
            });

            // Listen for elements going off screen (but still visible)
            let io_near = MyIntersectionObserver::new(1., {
                let scroll_at = scroll_at.clone();
                let eg = eg.clone();
                let get_child_coord = get_child_coord.clone();
                let get_scroll_coord = get_scroll_coord.clone();
                move |entries| eg.event(|pc| {
                    // Find the lowest intersecting->non entry (when seeking, the whole page will move
                    // off in one event).
                    let Some((_, e)) = entries.into_iter().filter_map(|entry| {
                        if entry.is_intersecting() {
                            return None;
                        }
                        if entry.intersection_ratio() == 0. {
                            return None;
                        }
                        let e = to_html_element(entry.target());
                        let top_coord = get_child_coord(&e);
                        if top_coord > get_scroll_coord() {
                            // Not on top-side
                            return None;
                        }
                        return Some((top_coord, e));
                    }).max_by(|a, b| f64::total_cmp(&a.0, &b.0)) else {
                        return;
                    };
                    scroll_at.set(pc, usize::from_str(&e.get_attribute(ATTR_INDEX).unwrap()).unwrap());
                }).unwrap()
            });

            // Hook up
            for child in &html_children {
                io_near.observe(&child);
                io_far.observe(&child);
            }
            let Some(iframe) = iframe.upgrade() else {
                return;
            };
            iframe.ref_own(|_| (io_near, io_far, EventListener::new(&idoc, "click", {
                let set_scroll_coord = set_scroll_coord.clone();
                let iframe = iframe.raw().dyn_into::<HtmlElement>().unwrap();
                move |_| {
                    set_scroll_coord(get_scroll_coord() + iframe.get_bounding_client_rect().height() * 4. / 5.);
                }
            })));
            eg.event(|pc| {
                iframe.ref_own(|_| (
                    //. .
                    link!(
                        (pc = pc),
                        (external_at = external_at.clone()),
                        (scroll_at = scroll_at.clone()),
                        (
                            set_scroll_coord = set_scroll_coord.clone(),
                            get_child_coord = get_child_coord.clone(),
                            html_children = html_children,
                        ),
                        {
                            let seek = external_at.borrow().min(html_children.len());
                            scroll_at.set(pc, seek);
                            set_scroll_coord(get_child_coord(&html_children[seek]));
                        }
                    ),
                    link!((pc = pc), (scroll_at = scroll_at.clone()), (external_at = external_at.clone()), (), {
                        external_at.set(pc, *scroll_at.borrow());
                    }),
                ));
            }).unwrap();
            _ = seekable.send(true);
        }
    }));
}

impl PlaylistMedia for PlaylistMediaBook {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, log: &Rc<dyn Log>, pc: &mut ProcessingContext) -> El {
        _ = self.seekable.send(false);
        let iframe = el("iframe").attr("src", &self.url);
        iframe
            .html()
            .style()
            .set_property("pointer-events", "initial")
            .log(log, "Error setting iframe pointer-events");
        iframe.ref_own(|_| EventListener::once(&iframe.raw(), "load", {
            let iframe = iframe.weak();
            let length = self.length.clone();
            let seekable = self.seekable.clone();
            let external_at = self.at.clone();
            let eg = pc.eg();
            let log = log.clone();
            move |_| {
                let Some(iframe) = iframe.upgrade() else {
                    return;
                };
                let Some(idoc) = iframe.raw().dyn_into::<HtmlIFrameElement>().unwrap().content_document() else {
                    log.log("Iframe missing contentDocument, can't show book media");
                    return;
                };
                let scroll_at = Prim::new(*external_at.borrow());
                if document().ready_state() == "loading" {
                    iframe.ref_own(|_| EventListener::once(&idoc, "DOMContentLoaded", {
                        let iframe = iframe.weak();
                        let idoc = idoc.clone();
                        let log = log.clone();
                        move |_| {
                            let Some(iframe) = iframe.upgrade() else {
                                return;
                            };
                            setup_book_idoc(&log, eg, iframe, idoc, external_at, scroll_at, length, seekable);
                        }
                    }));
                } else {
                    setup_book_idoc(&log, eg, iframe, idoc, external_at, scroll_at, length, seekable);
                }
            }
        }));
        return iframe;
    }

    fn pm_play(&self, _log: &Rc<dyn Log>) { }

    fn pm_stop(&self) { }

    fn pm_get_max_time(&self) -> Option<f64> {
        return self.length.get().map(|x| x as f64);
    }

    fn pm_get_time(&self) -> f64 {
        return *self.at.borrow() as f64;
    }

    fn pm_format_time(&self, time: f64) -> String {
        return format!("{}", time as u64);
    }

    fn pm_seek(&self, pc: &mut ProcessingContext, time: f64) {
        self.at.set(pc, time as usize);
    }

    fn pm_preload(&self, _log: &Rc<dyn Log>, _env: &Env) { }

    fn pm_unpreload(&self, _log: &Rc<dyn Log>) { }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let mut seekable = WatchStream::new(self.seekable.subscribe());
        return async move {
            while let Some(false) = seekable.next().await { }
        }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

pub async fn pm_share_ready_prep(
    eg: EventGraph,
    log: &Rc<dyn Log>,
    env: &Env,
    media: &dyn PlaylistMedia,
    new_time: f64,
) {
    media.pm_preload(log, env);
    media.pm_wait_until_seekable().await;
    eg.event(|pc| {
        media.pm_seek(pc, new_time);
    }).unwrap();
    media.pm_wait_until_buffered().await;
}
