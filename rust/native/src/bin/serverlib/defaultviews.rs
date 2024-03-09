use shared::model::{
    view::{
        Align,
        FieldOrLiteral,
        Layout,
        LayoutIndividual,
        LayoutTable,
        LineSizeMode,
        Orientation,
        QueryOrField,
        ViewPartList,
        Widget,
        WidgetMediaButton,
        WidgetImage,
        WidgetNest,
        WidgetTextLine,
    },
    View,
};

pub fn default_view_albums() -> View {
    return View {
        name: "Albums".to_string(),
        parameters: vec![],
        def: ViewPartList {
            data: QueryOrField::Query(include_str!("query_albums.cozo").to_string()),
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
                            width: "5cm".to_string(),
                            height: "".to_string(),
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
                                    size_max: "".to_string(),
                                    orientation: Orientation::RightDown,
                                    align: Align::Start,
                                }),
                                Widget::Sublist(ViewPartList {
                                    data: QueryOrField::Query(include_str!("query_tracks.cozo").to_string()),
                                    key_field: "file".to_string(),
                                    layout: Layout::Table(LayoutTable {
                                        orientation: Orientation::DownRight,
                                        align: Align::Start,
                                        x_scroll: true,
                                        columns: vec![
                                            //. .
                                            Widget::MediaButton(WidgetMediaButton {
                                                field: "file".to_string(),
                                                media_field: FieldOrLiteral::Field("media".to_string()),
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
                                                size_max: "".to_string(),
                                                orientation: Orientation::DownRight,
                                                align: Align::End,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Field("artist".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_mode: LineSizeMode::Wrap,
                                                size_max: "5cm".to_string(),
                                                orientation: Orientation::DownRight,
                                                align: Align::Start,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Literal(" - ".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_max: "".to_string(),
                                                size_mode: LineSizeMode::Wrap,
                                                orientation: Orientation::DownRight,
                                                align: Align::Start,
                                            }),
                                            Widget::TextLine(WidgetTextLine {
                                                data: FieldOrLiteral::Field("name".to_string()),
                                                prefix: "".to_string(),
                                                suffix: "".to_string(),
                                                size: "12pt".to_string(),
                                                size_max: "5cm".to_string(),
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
        },
    };
}
