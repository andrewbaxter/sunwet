use {
    crate::playlist::{
        playlist_toggle_play,
        PlaylistState,
    },
    lunk::{
        link,
        ProcessingContext,
    },
    rooting::{
        el,
        El,
    },
    shared::interface::config::view::Align,
    std::cell::Cell,
    crate::el_general::{
        el_button_icon_blank,
        el_err_block,
        el_icon,
        CSS_ERROR,
        ICON_TRANSPORT_PAUSE,
        ICON_TRANSPORT_PLAY,
    },
};

pub const CSS_TREE: &'static str = "tree";
pub const CSS_TREE_NEST: &'static str = "tree_nest";
pub const CSS_TREE_LAYOUT_INDIVIDUAL: &'static str = "tree_layout_individual";
pub const CSS_TREE_LAYOUT_TABLE: &'static str = "tree_layout_table";
pub const CSS_TREE_TEXT: &'static str = "tree_text";
pub const CSS_TREE_IMAGE: &'static str = "tree_image";
pub const CSS_TREE_MEDIA_BUTTON: &'static str = "tree_media_button";

pub fn el_image_err(text: String) -> El {
    return el_err_block(text).classes(&["image"]);
}

pub fn el_media_button(pc: &mut ProcessingContext, state: &PlaylistState, entry: usize) -> El {
    return el_button_icon_blank(pc, {
        let state = state.clone();
        move |pc| {
            playlist_toggle_play(pc, &state, Some(entry));
        }
    }).own(|e| link!(
        //. .
        (_pc = pc),
        (playing = state.0.playing.clone(), playing_i = state.0.playing_i.clone()),
        (),
        (button = e.weak(), entry = entry, previous = Cell::new(None)) {
            let button = button.upgrade()?;
            let new_playing = *playing.borrow() && *playing_i.borrow().as_ref().unwrap() == *entry;
            if previous.get() != Some(new_playing) {
                button.ref_clear();
                if new_playing {
                    button.ref_push(el_icon(ICON_TRANSPORT_PAUSE)).ref_attr("title", "Pause");
                } else {
                    button.ref_push(el_icon(ICON_TRANSPORT_PLAY)).ref_attr("title", "Play");
                }
            }
            previous.set(Some(new_playing));
        }
    ));
}

pub fn el_media_button_err(text: String) -> El {
    return el("div").classes(&[CSS_ERROR]).text(&text);
}

pub fn style_tree(type_: &str, depth: usize, align: Align, widget: &El) {
    widget.ref_classes(&[
        //. .
        CSS_TREE,
        type_,
        &format!("tree_depth_{}", depth),
        if depth % 2 == 0 {
            "tree_depth_even"
        } else {
            "tree_depth_odd"
        },
        match align {
            Align::Start => "align_start",
            Align::Middle => "align_middle",
            Align::End => "align_end",
        },
    ]);
}
