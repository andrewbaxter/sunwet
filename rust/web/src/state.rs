use std::{
    cell::RefCell,
    collections::HashMap,
    pin::Pin,
    rc::Rc,
};
use futures::{
    future::{
        BoxFuture,
        Shared,
    },
    Future,
};
use lunk::{
    Prim,
    ProcessingContext,
};
use rooting::{
    el,
    El,
    WeakEl,
};
use rooting_forms::BigString;
use serde::{
    Deserialize,
    Serialize,
};
use crate::{
    ministate::{
        record_new_ministate,
        Ministate,
    },
    page_query::{
        build_page_view,
        build_page_view_by_id,
        definition::{
            Align,
            Layout,
            LayoutIndividual,
            Orientation,
            QueryOrField,
            WidgetList,
            WidgetNest,
        },
    },
    playlist::PlaylistState,
    util::BgVal,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct View {
    pub name: String,
    pub def: WidgetList,
}

pub struct State_ {
    pub origin: String,
    pub playlist: PlaylistState,
    pub views: BgVal<Rc<RefCell<HashMap<usize, View>>>>,
    pub mobile_vert_title_group: WeakEl,
    pub title_group: WeakEl,
    pub body_group: WeakEl,
}

pub type State = Rc<State_>;

pub fn el_ministate_button(
    pc: &mut ProcessingContext,
    state: &State,
    icon: &str,
    text: &str,
    ministate: Ministate,
) -> El {
    return el("a")
        .attr("href", &format!("#{}", serde_json::to_string(&ministate).unwrap()))
        .push(el("div").text(icon))
        .push(el("span").text(text))
        .on("click", {
            let eg = pc.eg();
            let state = state.clone();
            move |ev| eg.event(|pc| {
                ev.stop_propagation();
                change_ministate(pc, &state, &ministate);
            })
        });
}

pub fn build_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    match s {
        Ministate::Home => {
            state.mobile_vert_title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text("Sunwet"));
            state.title_group.upgrade().unwrap().ref_clear().ref_push(el("h1").text("Sunwet"));
            state.body_group.upgrade().unwrap().ref_clear();
        },
        Ministate::View { id, title, play_entry, play_time } => {
            build_page_view_by_id(pc, state, &title, *id, play_entry.clone(), *play_time);
        },
        Ministate::NewView => {
            build_page_view(pc, state, View {
                name: "New view".to_string(),
                def: WidgetList {
                    data: QueryOrField::Query(BigString("".to_string())),
                    layout: Layout::Individual(LayoutIndividual {
                        orientation: Orientation::DownRight,
                        align: Align::Start,
                        item: WidgetNest {
                            orientation: Orientation::DownRight,
                            align: Align::Start,
                            children: vec![],
                        },
                    }),
                },
            }, vec![], 0.);
        },
    }
}

pub fn change_ministate(pc: &mut ProcessingContext, state: &State, s: &Ministate) {
    record_new_ministate(s);
    build_ministate(pc, state, s);
}
