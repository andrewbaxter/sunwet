use {
    crate::js::{
        async_event,
        LogJsErr,
    },
    futures::FutureExt,
    rooting::El,
    std::{
        future::Future,
        pin::Pin,
    },
    wasm_bindgen::JsCast,
    web_sys::HtmlMediaElement,
};

fn pm_wait_until_buffered(m: HtmlMediaElement) -> Pin<Box<dyn Future<Output = ()>>> {
    return async move {
        // 4 = `HAVE_ENOUGH_DATA`
        if m.ready_state() < 4 {
            // ios doesn't load until you manually tell it to load, even if preload is set to
            // auto. This may not be needed with the seek workaround (rare case of two wrongs
            // making just one wrong).
            m.load();
            async_event(&m, "canplaythrough").await;
        }
    }.boxed_local();
}

fn pm_wait_until_seekable(m: HtmlMediaElement) -> Pin<Box<dyn Future<Output = ()>>> {
    return async move {
        // 1 = `HAVE_METADATA`
        if m.ready_state() < 1 {
            async_event(&m, "loadedmetadata").await;
        }
    }.boxed_local();
}

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
    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>>;
    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>>;
}

pub struct PlaylistMediaAudio {
    pub element: El,
}

impl PlaylistMediaAudio {
    fn pm_media(&self) -> HtmlMediaElement {
        return self.element.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for PlaylistMediaAudio {
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

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return pm_wait_until_seekable(self.pm_media().clone());
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return pm_wait_until_buffered(self.pm_media().clone());
    }
}

pub struct PlaylistMediaVideo {
    pub element: El,
}

impl PlaylistMediaVideo {
    fn pm_media(&self) -> HtmlMediaElement {
        return self.element.raw().dyn_ref::<HtmlMediaElement>().unwrap().to_owned();
    }
}

impl PlaylistMedia for PlaylistMediaVideo {
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

    fn pm_wait_until_seekable(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return pm_wait_until_seekable(self.pm_media().clone());
    }

    fn pm_wait_until_buffered(&self) -> Pin<Box<dyn Future<Output = ()>>> {
        return pm_wait_until_buffered(self.pm_media().clone());
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
