use {
    crate::js::{
        async_event,
        LogJsErr,
    },
    futures::FutureExt,
    rooting::{
        El,
    },
    std::{
        cell::RefCell,
        future::Future,
        pin::Pin,
    },
    wasm_bindgen::{
        JsCast,
    },
    web_sys::{
        HtmlMediaElement,
    },
};

pub trait PlaylistMedia {
    fn pm_display(&self) -> bool;
    fn pm_play(&self);
    fn pm_stop(&self);
    fn pm_get_time(&self) -> f64;
    fn pm_get_max_time(&self) -> Option<f64>;
    fn pm_seek(&self, time: f64);
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

    fn pm_seek(&self, time: f64) {
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

    fn pm_seek(&self, _time: f64) { }

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

pub async fn pm_ready_prep(media: &dyn PlaylistMedia, new_time: f64) {
    media.pm_preload();
    media.pm_wait_until_seekable().await;
    media.pm_seek(new_time);
    media.pm_wait_until_buffered().await;
}
