use {
    crate::{
        js::{
            async_event,
            el_async,
            ElExt,
            LogJsErr,
        },
    },
    flowcontrol::ta_return,
    futures::FutureExt,
    gloo::{
        events::EventListener,
        timers::{
            callback::Timeout,
            future::TimeoutFuture,
        },
        utils::window,
    },
    js_sys::{
        Array,
    },
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
    tokio::sync::{
        Notify,
    },
    wasm_bindgen::{
        prelude::Closure,
        JsCast,
        JsValue,
    },
    web_sys::{
        Element,
        HtmlElement,
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
    fn pm_preload(&self);
    fn pm_unpreload(&self);
    fn pm_el(&self) -> &El;
    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct PlaylistMediaAudioVideo {
    pub display: bool,
    pub media_el: HtmlMediaElement,
    pub el: El,
    pub loaded_src: RefCell<Option<String>>,
}

impl PlaylistMediaAudioVideo {
    pub fn new_audio(el: El) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            display: false,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            loaded_src: RefCell::new(None),
        };
    }

    pub fn new_video(el: El) -> PlaylistMediaAudioVideo {
        return PlaylistMediaAudioVideo {
            display: true,
            media_el: el.raw().dyn_into().unwrap(),
            el: el,
            loaded_src: RefCell::new(None),
        };
    }
}

impl PlaylistMedia for PlaylistMediaAudioVideo {
    fn pm_display(&self) -> bool {
        return self.display;
    }

    fn pm_el(&self) -> &El {
        return &self.el;
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

    fn pm_preload(&self) {
        self.media_el.set_attribute("preload", "auto").log("Error setting preload attribute");
        let current_src = self.media_el.current_src();
        if self.loaded_src.borrow_mut().as_ref() != Some(&current_src) {
            // iOS doesn't load unless load called. Load resets currentTime to 0 on chrome
            // (desktop/android).
            self.media_el.load();
            *self.loaded_src.borrow_mut() = Some(current_src);
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
}

impl PlaylistMediaImage { }

impl PlaylistMedia for PlaylistMediaImage {
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

    fn pm_format_time(&self, time: f64) -> String {
        return format!("{}", time as usize);
    }

    fn pm_seek(&self, _pc: &mut ProcessingContext, _time: f64) { }

    fn pm_preload(&self) {
        self.element.ref_attr("loading", "eager");
    }

    fn pm_unpreload(&self) {
        self.element.ref_attr("loading", "auto");
    }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

const ATTR_INDEX: &str = "data-index";

pub struct PlaylistMediaComic {
    pub length: Rc<Cell<Option<usize>>>,
    pub seekable: Rc<Notify>,
    pub at: Prim<usize>,
    pub element: El,
}

impl PlaylistMediaComic {
    pub fn new<
        F: Future<Output = Result<ComicManifest, String>>,
        C: 'static + FnOnce(String) -> F,
    >(pc: &mut ProcessingContext, url: &str, req_manifest: C, restore_index: usize) -> Self {
        const PRE_POST_H_PAD: &str = "50dvw";
        let seekable = Rc::new(Notify::new());
        let at = Prim::new(restore_index);
        let length = Rc::new(Cell::new(None));
        let eg = pc.eg();
        let url = url.to_string();
        return Self {
            seekable: seekable.clone(),
            length: length.clone(),
            at: at.clone(),
            element: el_async(async move {
                ta_return!(Vec < El >, String);
                let manifest = req_manifest(url.clone()).await?;
                let rtl = manifest.rtl;

                // Outer container - takes strut height but keeps parent width; overlaps to avoid
                // strut affecting page layout
                let outer = el("div");
                let outer_style = outer.html().style();
                outer_style.set_property("width", "100%").log("Error setting outer style attr width");
                outer_style.set_property("height", "max-content").log("Error setting outer style attr height");
                outer_style.set_property("maxHeight", "100%").log("Error setting outer style attr maxHeight");
                outer_style.set_property("display", "grid").log("Error setting outer style attr display");
                outer_style
                    .set_property("gridTemplateColumns", "1fr")
                    .log("Error setting outer style attr gridTemplateColumns");
                outer_style
                    .set_property("gridTemplateRows", "1fr")
                    .log("Error setting outer style attr gridTemplateRows");
                outer_style.set_property("position", "relative").log("Error setting outer style attr position");
                outer_style.set_property("overflow", "hidden").log("Error setting outer style attr overflow");

                // Limits height to show full page aspect ratio
                let strut = el("div");
                outer.ref_push(strut.clone());
                let strut_style = strut.html().style();
                strut_style.set_property("maxWidth", "100%").log("Error setting strut style attr maxWidth");
                strut_style.set_property("gridColumn", "1").log("Error setting strut style attr gridColumn");
                strut_style.set_property("gridRow", "1").log("Error setting strut style attr gridRow");

                // Actual visible stuff, scrolls and adopts size from parent
                let inner = el("div");
                let inner_style = inner.html().style();
                outer.ref_push(inner.clone());
                inner_style.set_property("gridColumn", "1").log("Error setting inner attr gridColumn");
                inner_style.set_property("gridRow", "1").log("Error setting inner attr gridRow");
                inner_style.set_property("position", "absolute").log("Error setting inner attr position");
                inner_style.set_property("left", "0").log("Error setting inner attr left");
                inner_style.set_property("right", "0").log("Error setting inner attr right");
                inner_style.set_property("top", "0").log("Error setting inner attr top");
                inner_style.set_property("bottom", "0").log("Error setting inner attr bottom");
                inner_style.set_property("display", "flex").log("Error setting inner attr display");
                inner_style.set_property("overflowX", "scroll").log("Error setting inner style attr overflowX");
                if rtl {
                    inner_style
                        .set_property("flexDirection", "row-reverse")
                        .log("Error setting inner style attr flowDirection");
                } else {
                    inner_style
                        .set_property("flexDirection", "row")
                        .log("Error setting inner style attr flowDirection");
                }

                // Populate pages, gather page info, organize
                struct PageLookupEntry {
                    page_in_group: usize,
                    group_in_media: usize,
                }

                struct State {
                    at: Prim<usize>,
                    inner: El,
                    groups: Vec<Vec<El>>,
                    page_lookup: HashMap<usize, PageLookupEntry>,
                    wip_group: Vec<El>,
                }

                impl State {
                    fn build_flush(&mut self) {
                        if self.wip_group.is_empty() {
                            return;
                        }
                        self.groups.push(self.wip_group.split_off(0));
                    }

                    fn view_width(&self) -> f64 {
                        self.inner.html().get_bounding_client_rect().width()
                    }

                    fn set_scroll_center(&self, want_center: f64) {
                        self.inner.ref_attr("scrollLeft", &format!("{}", want_center - self.view_width() / 2.));
                    }

                    fn get_scroll_center(&self) -> f64 {
                        return f64::from_str(&self.inner.html().get_attribute("scrollLeft").unwrap()).unwrap() +
                            self.view_width() / 2.;
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
                            return f64::from_str(&group[0].html().get_attribute("offsetLeft").unwrap()).unwrap() +
                                group_width / 2.;
                        } else {
                            let e = &group[entry.page_in_group].html();
                            return f64::from_str(&e.get_attribute("offsetLeft").unwrap()).unwrap() +
                                e.get_bounding_client_rect().width() / 2.;
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

                let mut build_pages = State {
                    at: at.clone(),
                    inner: inner,
                    groups: Default::default(),
                    page_lookup: Default::default(),
                    wip_group: Default::default(),
                };
                let mut min_aspect = 1.;
                for (i, page) in manifest.pages.iter().enumerate() {
                    let img = el("img");
                    img.ref_attr(ATTR_INDEX, &i.to_string());
                    img.ref_attr("loading", "lazy");
                    img.ref_attr("src", &format!("{}/{}", url, page.path));
                    let img_style = img.html().style();
                    img_style.set_property("height", "100%").log("Error setting page style prop height");
                    img_style
                        .set_property("aspectRatio", &format!("{}/{}", page.width, page.height))
                        .log("Error setting page style prop aspectRatio");
                    let vert_aspect = page.width as f64 / page.height as f64;
                    if vert_aspect < min_aspect {
                        min_aspect = vert_aspect;
                    }

                    // Pre-group pad
                    if i == 0 {
                        let pad = el("div");
                        pad
                            .html()
                            .style()
                            .set_property("minWidth", PRE_POST_H_PAD)
                            .log("Error setting pad style property minWidth");
                        build_pages.inner.ref_push(pad);
                    } else if build_pages.wip_group.is_empty() {
                        let pad = el("div");
                        pad
                            .html()
                            .style()
                            .set_property("minWidth", "1cm")
                            .log("Error setting pad style property minWidth");
                        build_pages.inner.ref_push(pad);
                    }

                    // File page
                    if page.width > page.height {
                        build_pages.build_flush();
                    }
                    build_pages.page_lookup.insert(i, PageLookupEntry {
                        page_in_group: build_pages.wip_group.len(),
                        group_in_media: build_pages.groups.len(),
                    });
                    build_pages.wip_group.push(img.clone());
                    if page.width > page.height || i == 0 || build_pages.wip_group.len() == 2 {
                        build_pages.build_flush();
                    }
                    build_pages.inner.ref_push(img);

                    // Final page post-pad
                    if i == manifest.pages.len() - 1 {
                        let pad = el("div");
                        pad
                            .html()
                            .style()
                            .set_property("minWidth", PRE_POST_H_PAD)
                            .log("Error setting pad style property minWidth");
                        build_pages.inner.ref_push(pad);
                    }
                }
                strut_style
                    .set_property("aspectRatio", &min_aspect.to_string())
                    .log("Error setting strut style property aspectRatio");
                build_pages.build_flush();
                let state = Rc::new(build_pages);

                // Wait for browser ready
                length.set(Some(manifest.pages.len()));
                outer.ref_own({
                    let outer = outer.weak();
                    move |_| spawn_rooted(async move {
                        loop {
                            let want_center = state.calc_want_center(restore_index);
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
                            let internal_at = Prim::new(restore_index);
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
                                        _ => { },
                                    }
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
                        }).unwrap();
                    })
                });
                return Ok(vec![outer]);
            }),
        };
    }
}

impl PlaylistMedia for PlaylistMediaComic {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self) -> &El {
        return &self.element;
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
        return format!("{}", time as usize);
    }

    fn pm_seek(&self, pc: &mut ProcessingContext, time: f64) {
        self.at.set(pc, time as usize);
    }

    fn pm_preload(&self) { }

    fn pm_unpreload(&self) { }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let seekable = self.seekable.clone();
        return async move {
            seekable.notified().await;
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
            let mut o = IntersectionObserverInit::new();
            o.threshold(&JsValue::from(threshold));
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
    pub seekable: Rc<Notify>,
    pub at: Prim<usize>,
    pub element: El,
}

impl PlaylistMediaBook {
    pub fn new(pc: &mut ProcessingContext, url: &str, restore_index: usize) -> Self {
        let seekable = Rc::new(Notify::new());
        let at = Prim::new(restore_index);
        let length = Rc::new(Cell::new(None));
        let iframe = el("iframe").attr("src", &format!("{}/index.html", url));
        let idoc = iframe.raw().dyn_into::<HtmlIFrameElement>().unwrap().content_document().unwrap();
        iframe.ref_own(|_| EventListener::once(&idoc, "DOMContentLoaded", {
            let iframe = iframe.weak();
            let idoc = idoc.clone();
            let value = restore_index.clone();
            let length = length.clone();
            let seekable = seekable.clone();
            let external_at = at.clone();
            let internal_at = Prim::new(*external_at.borrow());
            let eg = pc.eg();
            move |_| {
                let Some(iframe) = iframe.upgrade() else {
                    return;
                };
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
                let html_children0 = idoc.query_selector_all("h1,h2,h3,h4,h5,h6,p,img").unwrap();
                let mut html_children = vec![];
                for i in 0 .. html_children0.length() {
                    let child = html_children0.item(i).unwrap().dyn_into::<HtmlElement>().unwrap();
                    child.set_attribute(ATTR_INDEX, &format!("{}", i)).log("Error setting book element index");
                    html_children.push(child);
                }
                length.set(Some(html_children.len()));
                iframe.ref_own(|iframe| spawn_rooted({
                    let iframe = iframe.weak();
                    async move {
                        // Do initial scroll restore - don't set up observers yet to avoid
                        // feedback/unnecessary noise
                        let restore_e = &html_children[value];
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
                        seekable.notify_one();
                    }
                }));
            }
        }));
        return PlaylistMediaBook {
            length: length,
            at: at,
            element: iframe,
            seekable: seekable,
        };
    }
}

impl PlaylistMedia for PlaylistMediaBook {
    fn pm_display(&self) -> bool {
        return true;
    }

    fn pm_el(&self) -> &El {
        return &self.element;
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

    fn pm_preload(&self) { }

    fn pm_unpreload(&self) { }

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        let seekable = self.seekable.clone();
        return async move {
            seekable.notified().await;
        }.boxed_local();
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return async { }.boxed_local();
    }
}

pub async fn pm_ready_prep(eg: EventGraph, media: &dyn PlaylistMedia, new_time: f64) {
    media.pm_preload();
    media.pm_wait_until_seekable().await;
    eg.event(|pc| {
        media.pm_seek(pc, new_time);
    }).unwrap();
    media.pm_wait_until_buffered().await;
}
