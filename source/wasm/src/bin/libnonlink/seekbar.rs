use {
    crate::libnonlink::{
        playlist::playlist_seek,
        state::state,
    },
    lunk::{
        link,
        Prim,
        ProcessingContext,
    },
    rooting::El,
    wasm_bindgen::JsCast,
    web_sys::{
        Element,
        Event,
        MouseEvent,
    },
};

pub fn setup_seekbar(pc: &mut ProcessingContext, seekbar: El, seekbar_fill: El, seekbar_label: El) {
    fn get_mouse_pct(ev: &Event) -> (f64, f64, MouseEvent) {
        let element = ev.target().unwrap().dyn_into::<Element>().unwrap();
        let ev = ev.dyn_ref::<MouseEvent>().unwrap();
        let element_rect = element.get_bounding_client_rect();
        let percent_x = ((ev.client_x() as f64 - element_rect.x()) / element_rect.width().max(0.001)).clamp(0., 1.);
        let percent_y = ((ev.client_y() as f64 - element_rect.y()) / element_rect.width().max(0.001)).clamp(0., 1.);
        return (percent_x, percent_y, ev.clone());
    }

    fn get_mouse_time(ev: &Event) -> Option<f64> {
        let Some(max_time) = *state().playlist.0.media_max_time.borrow() else {
            return None;
        };
        let percent = get_mouse_pct(ev).0;
        return Some(max_time * percent);
    }

    let hover_time = Prim::new(None);
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
    seekbar_fill.ref_attr("style", &format!("width: 0%;"));
    seekbar_fill.ref_own(|fill| link!(
        //. .
        (_pc = pc),
        (time = state().playlist.0.playing_time.clone(), max_time = state().playlist.0.media_max_time.clone()),
        (),
        (fill = fill.weak()) {
            let Some(max_time) = *max_time.borrow() else {
                return None;
            };
            let fill = fill.upgrade()?;
            fill.ref_attr("style", &format!("width: {}%;", *time.borrow() / max_time.max(0.0001) * 100.));
        }
    ));
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
            label.text(&state().playlist.format_time(time));
        }
    ));
}
