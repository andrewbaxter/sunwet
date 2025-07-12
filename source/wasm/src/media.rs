use {
    crate::{
        js::{
            self,
            async_event,
            el_async,
            env_preferred_audio_url,
            env_preferred_video_url,
            file_derivation_subtitles_url,
            log,
            style_export,
            ElExt,
            Env,
            LogJsErr,
        },
        world::file_url,
    },
    flowcontrol::ta_return,
    futures::{
        FutureExt,
        StreamExt,
    },
    gloo::{
        events::EventListener,
        timers::{
            callback::Timeout,
            future::TimeoutFuture,
        },
        utils::{
            document,
            window,
        },
    },
    js_sys::Array,
    lunk::{
        link,
        EventGraph,
        Prim,
        ProcessingContext,
    },
    rooting::{
        el,
        scope_any,
        spawn_rooted,
        El,
        ScopeValue,
    },
    shared::interface::{
        derived::ComicManifest,
        wire::link::SourceUrl,
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
    },
    tokio::sync::watch,
    tokio_stream::wrappers::WatchStream,
    wasm_bindgen::{
        prelude::Closure,
        JsCast,
        JsValue,
    },
    web_sys::{
        Document,
        Element,
        HtmlElement,
        HtmlHeadingElement,
        HtmlIFrameElement,
        HtmlMediaElement,
        IntersectionObserver,
        IntersectionObserverEntry,
        IntersectionObserverInit,
        KeyboardEvent,
        MouseEvent,
        WheelEvent,
    },
};

pub trait PlaylistMedia {
    /// Audio - no display, video/image show fs overlay based on element
    fn pm_display(&self) -> bool;

    /// Transition to auto-advancing
    fn pm_play(&self);
    fn pm_stop(&self);
    fn pm_get_time(&self) -> f64;
    fn pm_get_max_time(&self) -> Option<f64>;
    fn pm_format_time(&self, time: f64) -> String;
    fn pm_seek(&self, pc: &mut ProcessingContext, time: f64);
    fn pm_preload(&self, env: &Env);
    fn pm_unpreload(&self);
    fn pm_el(&self, pc: &mut ProcessingContext) -> El;
    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct PlaylistMediaAudioVideo {
    pub video: bool,
    pub media_el: HtmlMediaElement,
    pub el: El,
    pub src: SourceUrl,
    pub loaded_src: RefCell<Option<String>>,
}

impl PlaylistMediaAudioVideo {
    pub fn new_audio(el: El, src: SourceUrl) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            video: false,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            src: src,
            loaded_src: RefCell::new(None),
        };
    }

    pub fn new_video(el: El, src: SourceUrl) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            video: true,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            src: src,
            loaded_src: RefCell::new(None),
        };
    }
}

impl PlaylistMedia for PlaylistMediaAudioVideo {
    fn pm_display(&self) -> bool {
        return self.video;
    }

    fn pm_el(&self, _pc: &mut ProcessingContext) -> El {
        return self.el.clone();
    }

    fn pm_play(&self) {
        self.media_el.play().log("Error playing video");
    }

    fn pm_stop(&self) {
        self.media_el.pause().unwrap();
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
        return self.media_el.current_time();
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
    }

    fn pm_preload(&self, env: &Env) {
        self.media_el.set_attribute("preload", "auto").log("Error setting preload attribute");
        let src = if self.video {
            match &self.src {
                SourceUrl::Url(v) => v.clone(),
                SourceUrl::File(v) => env_preferred_video_url(&env, &v),
            }
        } else {
            match &self.src {
                SourceUrl::Url(v) => v.clone(),
                SourceUrl::File(v) => env_preferred_audio_url(&env, &v),
            }
        };
        if src != self.media_el.current_src() {
            if self.video {
                match &self.src {
                    SourceUrl::Url(_) => { },
                    SourceUrl::File(v) => {
                        self.media_el.set_inner_html("");
                        for (i, lang) in env.languages.iter().enumerate() {
                            let track =
                                el("track")
                                    .attr("kind", "subtitles")
                                    .attr("src", &file_derivation_subtitles_url(&env, lang, &v))
                                    .attr("srclang", &lang);
                            if i == 0 {
                                track.ref_attr("default", "default");
                            }
                            self.media_el.append_child(&track.raw()).log("Error adding track to video element");
                        }
                    },
                }
            }
            self.media_el.set_src(&src);

            // iOS doesn't load unless load called. Load resets currentTime to 0 on chrome
            // (desktop/android). Avoid calling load again unless the url changes (to avoid
            // time reset).
            self.media_el.load();
        }
    }

    fn pm_unpreload(&self) {
        self.media_el.set_attribute("preload", "metadata").log("Error reducing preload attribute");
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
    pub src: SourceUrl,
}

impl PlaylistMediaImage { }

impl PlaylistMedia for PlaylistMediaImage {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, _pc: &mut ProcessingContext) -> El {
        return self.element.clone();
    }

    fn pm_play(&self) { }

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

    fn pm_preload(&self, env: &Env) {
        self.element.ref_attr("loading", "eager");
        self.element.ref_attr("src", &match &self.src {
            SourceUrl::Url(v) => v.clone(),
            SourceUrl::File(v) => file_url(&env, &v),
        });
    }

    fn pm_unpreload(&self) {
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
type PlaylistMediaComicReqManifestFn =
    Rc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<ComicManifest, String>>>>>;

pub struct PlaylistMediaComic {
    pub length: Rc<Cell<Option<usize>>>,
    pub seekable: watch::Sender<bool>,
    pub at: Prim<usize>,
    pub url: String,
    pub req_manifest: PlaylistMediaComicReqManifestFn,
}

impl PlaylistMediaComic {
    pub fn new(url: &str, req_manifest: PlaylistMediaComicReqManifestFn, restore_index: usize) -> Self {
        return Self {
            seekable: watch::channel(false).0,
            length: Rc::new(Cell::new(None)),
            at: Prim::new(restore_index),
            url: url.to_string(),
            req_manifest: req_manifest,
        };
    }
}

impl PlaylistMedia for PlaylistMediaComic {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, pc: &mut ProcessingContext) -> El {
        _ = self.seekable.send(false);
        let req_manifest = self.req_manifest.clone();
        let url = self.url.clone();
        let at = self.at.clone();
        let length = self.length.clone();
        let eg = pc.eg();
        let seekable = self.seekable.clone();

        // Super outer container, pads outer when height is reduced
        let root = style_export::cont_media_comic_outer(style_export::ContMediaComicOuterArgs { children: vec![] }).root;
        root.ref_push(el_async(async move {
            ta_return!(Vec < El >, String);
            let manifest = req_manifest(format!("{}/sunwet.json", url)).await?;
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
                    src: format!("{}/{}", url, page.path),
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
                    if new_index >= self.page_lookup.len() {
                        return;
                    }
                    self.set_scroll_center(self.calc_want_center(new_index));
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
                        let internal_at = Prim::new(*at.borrow());
                        state.inner.ref_own(|_| EventListener::new(&window(), "fullscreenchange", {
                            let state = Rc::downgrade(&state);
                            let internal_at = internal_at.clone();
                            move |_| {
                                let Some(state1) = state.upgrade() else {
                                    return;
                                };
                                *state1.fix_scroll.borrow_mut() = Some(spawn_rooted({
                                    let state = state.clone();
                                    let internal_at = internal_at.clone();
                                    async move {
                                        loop {
                                            {
                                                let Some(state) = state.upgrade() else {
                                                    return;
                                                };
                                                let want_center = state.calc_want_center(*internal_at.borrow());
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
                        }));
                        state.inner.ref_on("scroll", {
                            let visible = visible.clone();
                            let state = Rc::downgrade(&state);
                            let bg = Cell::new(None);
                            let eg = pc.eg();
                            let internal_at = internal_at.clone();
                            move |_| bg.set(Some(Timeout::new(300, {
                                let state = state.clone();
                                let eg = eg.clone();
                                let visible = visible.clone();
                                let internal_at = internal_at.clone();
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
                                                internal_at.set(pc, index);
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
                                (internal_at = internal_at.clone()),
                                (state = state.clone()),
                                {
                                    let seek = *external_at.borrow();
                                    internal_at.set(pc, seek);
                                    state.seek(pc, seek);
                                }
                            ),
                            link!(
                                (pc = pc),
                                (internal_at = internal_at.clone()),
                                (external_at = state.at.clone()),
                                (),
                                {
                                    external_at.set(pc, *internal_at.borrow());
                                }
                            ),
                        ));
                        outer.ref_own(|_| EventListener::new(&window(), "keydown", {
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

    fn pm_play(&self) { }

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

    fn pm_preload(&self, _env: &Env) { }

    fn pm_unpreload(&self) { }

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

struct MyIntersectionObserver_ {
    _root_cb: ScopeValue,
    o: IntersectionObserver,
}

impl Drop for MyIntersectionObserver_ {
    fn drop(&mut self) {
        self.o.disconnect();
    }
}

struct MyIntersectionObserver(Rc<MyIntersectionObserver_>);

impl MyIntersectionObserver {
    fn new(threshold: f64, mut cb: impl 'static + FnMut(Vec<IntersectionObserverEntry>)) -> Self {
        let scroll_observer_cb = Closure::new(move |entries: Array| {
            let entries =
                entries
                    .into_iter()
                    .map(|x| x.dyn_into::<IntersectionObserverEntry>().unwrap())
                    .collect::<Vec<_>>();
            cb(entries);
        });
        let scroll_observer = IntersectionObserver::new_with_options(scroll_observer_cb.as_ref().unchecked_ref(), &{
            let o = IntersectionObserverInit::new();
            o.set_threshold(&JsValue::from(threshold));
            o
        }).unwrap();
        return Self(Rc::new(MyIntersectionObserver_ {
            _root_cb: scope_any(scroll_observer_cb),
            o: scroll_observer,
        }));
    }
}

impl MyIntersectionObserver {
    fn observe(&self, e: &Element) {
        self.0.o.observe(e);
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

impl PlaylistMedia for PlaylistMediaBook {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self, pc: &mut ProcessingContext) -> El {
        _ = self.seekable.send(false);
        let iframe = el("iframe").attr("src", &format!("{}/index.html", self.url));
        iframe.html().style().set_property("pointer-events", "initial").log("Error setting iframe pointer-events");
        iframe.ref_own(|_| EventListener::once(&iframe.raw(), "load", {
            let iframe = iframe.weak();
            let length = self.length.clone();
            let seekable = self.seekable.clone();
            let external_at = self.at.clone();
            let eg = pc.eg();
            move |_| {
                let Some(iframe) = iframe.upgrade() else {
                    return;
                };
                let Some(idoc) = iframe.raw().dyn_into::<HtmlIFrameElement>().unwrap().content_document() else {
                    log("Iframe missing contentDocument, can't show book media");
                    return;
                };

                fn setup(
                    eg: EventGraph,
                    iframe: El,
                    idoc: Document,
                    external_at: Prim<usize>,
                    internal_at: Prim<usize>,
                    length: Rc<Cell<Option<usize>>>,
                    seekable: watch::Sender<bool>,
                ) {
                    // Wait for stuff to appear, not sure why this isn't handled by
                    // DOMContentLoaded... but I guess in some cases there might be js doing stuff too
                    let html_children0 = idoc.query_selector_all("h1,h2,h3,h4,h5,h6,p,img").unwrap();
                    if html_children0.length() == 0 {
                        log("Book iframe body has no typical document elements (h*,p,img), can't integrate");
                        return;
                    }
                    let mut html_children = vec![];
                    for i in 0 .. html_children0.length() {
                        let child0 = html_children0.item(i).unwrap();
                        js::log_js(
                            format!(
                                "iframe child {}; is element {}; is htmlelement {}; is headingelement {}",
                                i,
                                child0.is_instance_of::<Element>(),
                                child0.is_instance_of::<HtmlElement>(),
                                child0.is_instance_of::<HtmlHeadingElement>(),
                            ),
                            &child0,
                        );
                        let child = child0.dyn_into::<HtmlElement>().unwrap();
                        child.set_attribute(ATTR_INDEX, &format!("{}", i)).log("Error setting book element index");
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
                                    return c.get_bounding_client_rect().top() -
                                        ibody.get_bounding_client_rect().top();
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

                            // Start observing
                            //
                            // Listen for elements coming from off screen (newly visible)
                            let io_far = MyIntersectionObserver::new(0., {
                                let internal_at = internal_at.clone();
                                let eg = eg.clone();
                                let get_child_coord = get_child_coord.clone();
                                let get_scroll_coord = get_scroll_coord.clone();
                                move |entries| eg.event(|pc| {
                                    for entry in entries {
                                        if !entry.is_intersecting() {
                                            continue;
                                        }
                                        let e = entry.target().dyn_into::<HtmlElement>().unwrap();
                                        if get_child_coord(&e) > get_scroll_coord() {
                                            // Not at top
                                            continue;
                                        }
                                        internal_at.set(
                                            pc,
                                            usize::from_str(&e.get_attribute(ATTR_INDEX).unwrap()).unwrap(),
                                        );
                                    }
                                }).unwrap()
                            });

                            // Listen for elements going off screen (but still visible)
                            let io_near = MyIntersectionObserver::new(1., {
                                let internal_at = internal_at.clone();
                                let eg = eg.clone();
                                let get_child_coord = get_child_coord.clone();
                                let get_scroll_coord = get_scroll_coord.clone();
                                move |entries| eg.event(|pc| {
                                    for entry in entries {
                                        if entry.is_intersecting() {
                                            continue;
                                        }
                                        let e = entry.target().dyn_into::<HtmlElement>().unwrap();
                                        if get_child_coord(&e) > get_scroll_coord() {
                                            // Not at top
                                            continue;
                                        }
                                        internal_at.set(
                                            pc,
                                            usize::from_str(&e.get_attribute(ATTR_INDEX).unwrap()).unwrap(),
                                        );
                                    }
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
                            iframe.ref_on("click", {
                                let set_scroll_coord = set_scroll_coord.clone();
                                let iframe = iframe.raw().dyn_into::<HtmlElement>().unwrap();
                                move |_| {
                                    set_scroll_coord(
                                        get_scroll_coord() + iframe.get_bounding_client_rect().height() * 4. / 5.,
                                    );
                                }
                            });
                            eg.event(|pc| {
                                iframe.ref_own(|_| (
                                    //. .
                                    link!(
                                        (pc = pc),
                                        (external_at = external_at.clone()),
                                        (internal_at = internal_at.clone()),
                                        (
                                            set_scroll_coord = set_scroll_coord.clone(),
                                            get_child_coord = get_child_coord.clone(),
                                            html_children = html_children,
                                        ),
                                        {
                                            let seek = *external_at.borrow();
                                            internal_at.set(pc, seek);
                                            set_scroll_coord(get_child_coord(&html_children[seek]));
                                        }
                                    ),
                                    link!(
                                        (pc = pc),
                                        (internal_at = internal_at.clone()),
                                        (external_at = external_at.clone()),
                                        (),
                                        {
                                            external_at.set(pc, *internal_at.borrow());
                                        }
                                    ),
                                ));
                            }).unwrap();
                            _ = seekable.send(true);
                        }
                    }));
                }

                let internal_at = Prim::new(*external_at.borrow());
                if document().ready_state() == "loading" {
                    iframe.ref_own(|_| EventListener::once(&idoc, "DOMContentLoaded", {
                        let iframe = iframe.weak();
                        let idoc = idoc.clone();
                        move |_| {
                            let Some(iframe) = iframe.upgrade() else {
                                return;
                            };
                            setup(eg, iframe, idoc, external_at, internal_at, length, seekable);
                        }
                    }));
                } else {
                    setup(eg, iframe, idoc, external_at, internal_at, length, seekable);
                }
            }
        }));
        return iframe;
    }

    fn pm_play(&self) { }

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

    fn pm_preload(&self, _env: &Env) { }

    fn pm_unpreload(&self) { }

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

pub async fn pm_share_ready_prep(eg: EventGraph, env: &Env, media: &dyn PlaylistMedia, new_time: f64) {
    media.pm_preload(env);
    media.pm_wait_until_seekable().await;
    eg.event(|pc| {
        media.pm_seek(pc, new_time);
    }).unwrap();
    media.pm_wait_until_buffered().await;
}
