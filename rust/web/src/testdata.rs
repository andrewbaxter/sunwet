use std::{
    any::Any,
    cell::RefCell,
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
use crate::page_query::definition::{
    Align,
    BlockSizeMode,
    FieldOrLiteral,
    Layout,
    LayoutIndividual,
    LayoutTable,
    LineSizeMode,
    Orientation,
    QueryOrField,
    Widget,
    WidgetAudio,
    WidgetImage,
    WidgetList,
    WidgetNest,
    WidgetTextLine,
};

pub fn testdata_albums() -> WidgetList {
    return WidgetList {
        data: QueryOrField::Query(BigString(include_str!("query_albums.datalog").to_string())),
        key_field: "album_id".to_string(),
        layout: Layout::Individual(LayoutIndividual {
            orientation: Orientation::DownRight,
            align: Align::Start,
            x_scroll: false,
            item: WidgetNest {
                orientation: Orientation::RightDown,
                align: Align::Start,
                children: vec![
                    //. .
                    Widget::Image(WidgetImage {
                        data: FieldOrLiteral::Field("cover".to_string()),
                        size_mode: BlockSizeMode::Cover,
                        width: "5cm".to_string(),
                        height: "5cm".to_string(),
                        align: Align::Start,
                    }),
                    Widget::Nest(WidgetNest {
                        orientation: Orientation::DownRight,
                        align: Align::Start,
                        children: vec![
                            //. .
                            Widget::TextLine(WidgetTextLine {
                                data: FieldOrLiteral::Field("album".to_string()),
                                prefix: "".to_string(),
                                suffix: "".to_string(),
                                size: "14pt".to_string(),
                                size_mode: LineSizeMode::Ellipsize,
                                orientation: Orientation::RightDown,
                                align: Align::Start,
                            }),
                            Widget::Sublist(WidgetList {
                                data: QueryOrField::Query(
                                    BigString(include_str!("query_tracks.datalog").to_string()),
                                ),
                                key_field: "file".to_string(),
                                layout: Layout::Table(LayoutTable {
                                    orientation: Orientation::DownRight,
                                    align: Align::Start,
                                    x_scroll: true,
                                    columns: vec![
                                        //. .
                                        Widget::Audio(WidgetAudio {
                                            field: "file".to_string(),
                                            name_field: "name".to_string(),
                                            album_field: "album".to_string(),
                                            artist_field: "artist".to_string(),
                                            thumbnail_field: "cover".to_string(),
                                            align: Align::Start,
                                        }),
                                        Widget::TextLine(WidgetTextLine {
                                            data: FieldOrLiteral::Field("index".to_string()),
                                            prefix: "".to_string(),
                                            suffix: ".".to_string(),
                                            size: "12pt".to_string(),
                                            size_mode: LineSizeMode::Wrap,
                                            orientation: Orientation::DownRight,
                                            align: Align::End,
                                        }),
                                        Widget::TextLine(WidgetTextLine {
                                            data: FieldOrLiteral::Field("artist".to_string()),
                                            prefix: "".to_string(),
                                            suffix: "".to_string(),
                                            size: "12pt".to_string(),
                                            size_mode: LineSizeMode::Wrap,
                                            orientation: Orientation::DownRight,
                                            align: Align::Start,
                                        }),
                                        Widget::TextLine(WidgetTextLine {
                                            data: FieldOrLiteral::Literal(" - ".to_string()),
                                            prefix: "".to_string(),
                                            suffix: "".to_string(),
                                            size: "12pt".to_string(),
                                            size_mode: LineSizeMode::Wrap,
                                            orientation: Orientation::DownRight,
                                            align: Align::Start,
                                        }),
                                        Widget::TextLine(WidgetTextLine {
                                            data: FieldOrLiteral::Field("name".to_string()),
                                            prefix: "".to_string(),
                                            suffix: "".to_string(),
                                            size: "12pt".to_string(),
                                            size_mode: LineSizeMode::Wrap,
                                            orientation: Orientation::DownRight,
                                            align: Align::Start,
                                        })
                                    ],
                                }),
                            })
                        ],
                    })
                ],
            },
        }),
    };
}
