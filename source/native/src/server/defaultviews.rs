use {
    shared::interface::{
        config::view::{
            Align,
            FieldOrLiteral,
            Layout,
            LayoutIndividual,
            LayoutTable,
            LineSizeMode,
            Orientation,
            QueryOrField,
            View,
            ViewPartList,
            Widget,
            WidgetImage,
            WidgetMediaButton,
            WidgetNest,
            WidgetTextLine,
        },
        query::{
            Chain,
            FilterChainComparisonOperator,
            FilterExpr,
            FilterExprExists,
            FilterExprExistsType,
            MoveDirection,
            Query,
            Step,
            StepMove,
            StepRecurse,
            Subchain,
            Value,
        },
        triple::Node,
    },
};

pub fn node_is_album() -> Node {
    return Node::Id("sunwet/1/album".to_string());
}

pub fn node_is_track() -> Node {
    return Node::Id("sunwet/1/track".to_string());
}

pub const PREDICATE_IS: &str = "sunwet/1/is";
pub const PREDICATE_ELEMENT: &str = "sunwet/1/element";
pub const PREDICATE_INDEX: &str = "sunwet/1/index";
pub const PREDICATE_NAME: &str = "sunwet/1/name";
pub const PREDICATE_CREATOR: &str = "sunwet/1/creator";
pub const PREDICATE_COVER: &str = "sunwet/1/cover";
pub const PREDICATE_FILE: &str = "sunwet/1/file";
pub const PREDICATE_MEDIA: &str = "sunwet/1/media";
pub const ALBUMS_RECORD_KEY_ID: &str = "id";
pub const ALBUMS_RECORD_KEY_NAME: &str = "name";
pub const ALBUMS_RECORD_KEY_COVER: &str = "cover";
pub const ALBUMS_RECORD_KEY_ARTIST_ID: &str = "artist_id";
pub const ALBUMS_RECORD_KEY_ARTIST_NAME: &str = "artist_name";
pub const TRACKS_PARAM_ALBUM: &str = "album_id";
pub const TRACKS_RECORD_KEY_ID: &str = "id";
pub const TRACKS_RECORD_KEY_FILE: &str = "file";
pub const TRACKS_RECORD_KEY_NAME: &str = "name";
pub const TRACKS_RECORD_KEY_MEDIA: &str = "media";
pub const TRACKS_RECORD_KEY_INDEX: &str = "index";
pub const TRACKS_RECORD_KEY_ARTIST_ID: &str = "artist_id";
pub const TRACKS_RECORD_KEY_ARTIST_NAME: &str = "artist_name";

pub fn default_query_albums() -> Query {
    return Query {
        chain: Chain {
            subchain: Subchain {
                root: Some(Value::Literal(node_is_album())),
                steps: vec![Step::Move(StepMove {
                    dir: MoveDirection::Up,
                    predicate: PREDICATE_IS.to_string(),
                    first: false,
                    filter: None,
                })],
            },
            select: Some(ALBUMS_RECORD_KEY_ID.to_string()),
            children: vec![
                //. .
                Chain {
                    select: Some(ALBUMS_RECORD_KEY_NAME.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![
                            //. .
                            Step::Recurse(StepRecurse {
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Up,
                                        predicate: PREDICATE_ELEMENT.to_string(),
                                        first: false,
                                        filter: None,
                                    })],
                                },
                                first: false,
                            }),
                            Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_NAME.to_string(),
                                first: true,
                                filter: None,
                            })
                        ],
                    },
                    children: Default::default(),
                },
                Chain {
                    subchain: Subchain {
                        root: None,
                        steps: vec![
                            //. .
                            Step::Recurse(StepRecurse {
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Up,
                                        predicate: PREDICATE_ELEMENT.to_string(),
                                        first: false,
                                        filter: None,
                                    })],
                                },
                                first: false,
                            }),
                            Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_CREATOR.to_string(),
                                first: true,
                                filter: None,
                            })
                        ],
                    },
                    select: Some(ALBUMS_RECORD_KEY_ARTIST_ID.to_string()),
                    children: vec![Chain {
                        select: Some(ALBUMS_RECORD_KEY_ARTIST_NAME.to_string()),
                        subchain: Subchain {
                            root: None,
                            steps: vec![
                                //. .
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: PREDICATE_ELEMENT.to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: PREDICATE_NAME.to_string(),
                                    first: true,
                                    filter: None,
                                })
                            ],
                        },
                        children: Default::default(),
                    }],
                },
                Chain {
                    select: Some(ALBUMS_RECORD_KEY_COVER.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![
                            //. .
                            Step::Recurse(StepRecurse {
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Up,
                                        predicate: PREDICATE_ELEMENT.to_string(),
                                        first: false,
                                        filter: None,
                                    })],
                                },
                                first: false,
                            }),
                            Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_COVER.to_string(),
                                first: true,
                                filter: None,
                            })
                        ],
                    },
                    children: Default::default(),
                }
            ],
        },
        sort: vec![],
    };
}

pub fn default_query_album_tracks() -> Query {
    return Query {
        chain: Chain {
            subchain: Subchain {
                root: Some(Value::Parameter(TRACKS_PARAM_ALBUM.to_string())),
                steps: vec![Step::Move(StepMove {
                    dir: MoveDirection::Down,
                    predicate: PREDICATE_ELEMENT.to_string(),
                    first: false,
                    filter: Some(FilterExpr::Exists(FilterExprExists {
                        type_: FilterExprExistsType::Exists,
                        subchain: Subchain {
                            root: None,
                            steps: vec![Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_IS.to_string(),
                                filter: None,
                                first: false,
                            })],
                        },
                        filter: Some((FilterChainComparisonOperator::Eq, Value::Literal(node_is_track()))),
                    })),
                })],
            },
            select: Some(TRACKS_RECORD_KEY_ID.to_string()),
            children: vec![
                //. .
                Chain {
                    select: Some(ALBUMS_RECORD_KEY_NAME.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![
                            //. .
                            Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_NAME.to_string(),
                                first: true,
                                filter: None,
                            })
                        ],
                    },
                    children: Default::default(),
                },
                Chain {
                    select: Some(TRACKS_RECORD_KEY_ARTIST_ID.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![
                            //. .
                            Step::Recurse(StepRecurse {
                                subchain: Subchain {
                                    root: None,
                                    steps: vec![Step::Move(StepMove {
                                        dir: MoveDirection::Up,
                                        predicate: PREDICATE_ELEMENT.to_string(),
                                        first: false,
                                        filter: None,
                                    })],
                                },
                                first: false,
                            }),
                            Step::Move(StepMove {
                                dir: MoveDirection::Down,
                                predicate: PREDICATE_CREATOR.to_string(),
                                first: true,
                                filter: None,
                            })
                        ],
                    },
                    children: vec![Chain {
                        select: Some(TRACKS_RECORD_KEY_ARTIST_NAME.to_string()),
                        subchain: Subchain {
                            root: None,
                            steps: vec![
                                //. .
                                Step::Recurse(StepRecurse {
                                    subchain: Subchain {
                                        root: None,
                                        steps: vec![Step::Move(StepMove {
                                            dir: MoveDirection::Up,
                                            predicate: PREDICATE_ELEMENT.to_string(),
                                            first: false,
                                            filter: None,
                                        })],
                                    },
                                    first: false,
                                }),
                                Step::Move(StepMove {
                                    dir: MoveDirection::Down,
                                    predicate: PREDICATE_NAME.to_string(),
                                    first: true,
                                    filter: None,
                                })
                            ],
                        },
                        children: Default::default(),
                    }],
                },
                Chain {
                    select: Some(TRACKS_RECORD_KEY_INDEX.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: PREDICATE_INDEX.to_string(),
                            first: true,
                            filter: None,
                        })],
                    },
                    children: Default::default(),
                },
                Chain {
                    select: Some(TRACKS_RECORD_KEY_FILE.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: PREDICATE_FILE.to_string(),
                            first: true,
                            filter: None,
                        })],
                    },
                    children: Default::default(),
                },
                Chain {
                    select: Some(TRACKS_RECORD_KEY_MEDIA.to_string()),
                    subchain: Subchain {
                        root: None,
                        steps: vec![Step::Move(StepMove {
                            dir: MoveDirection::Down,
                            predicate: PREDICATE_MEDIA.to_string(),
                            first: true,
                            filter: None,
                        })],
                    },
                    children: Default::default(),
                }
            ],
        },
        sort: vec![],
    };
}
//. pub fn default_view_albums() -> View {
//.     return View {
//.         allow_target: IAM_TARGET_WORLD_RO,
//.         id: "albums".to_string(),
//.         name: "Albums".to_string(),
//.         parameters: vec![],
//.         def: ViewPartList {
//.             data: QueryOrField::Query(default_query_albums()),
//.             key_field: ALBUMS_RECORD_KEY_ID.to_string(),
//.             layout: Layout::Individual(LayoutIndividual {
//.                 orientation: Orientation::DownRight,
//.                 align: Align::Start,
//.                 x_scroll: false,
//.                 item: WidgetNest {
//.                     orientation: Orientation::RightDown,
//.                     align: Align::Start,
//.                     children: vec![
//.                         //. .
//.                         Widget::Image(WidgetImage {
//.                             data: FieldOrLiteral::Field(ALBUMS_RECORD_KEY_COVER.to_string()),
//.                             width: "5cm".to_string(),
//.                             height: "".to_string(),
//.                             align: Align::Start,
//.                         }),
//.                         Widget::Nest(WidgetNest {
//.                             orientation: Orientation::DownRight,
//.                             align: Align::Start,
//.                             children: vec![
//.                                 //. .
//.                                 Widget::TextLine(WidgetTextLine {
//.                                     data: FieldOrLiteral::Field(ALBUMS_RECORD_KEY_NAME.to_string()),
//.                                     prefix: "".to_string(),
//.                                     suffix: "".to_string(),
//.                                     size: "14pt".to_string(),
//.                                     size_mode: LineSizeMode::Ellipsize,
//.                                     size_max: "".to_string(),
//.                                     orientation: Orientation::RightDown,
//.                                     align: Align::Start,
//.                                 }),
//.                                 Widget::Sublist(ViewPartList {
//.                                     data: QueryOrField::Query(default_query_album_tracks()),
//.                                     key_field: TRACKS_RECORD_KEY_ID.to_string(),
//.                                     layout: Layout::Table(LayoutTable {
//.                                         orientation: Orientation::DownRight,
//.                                         align: Align::Start,
//.                                         x_scroll: true,
//.                                         columns: vec![
//.                                             //. .
//.                                             Widget::MediaButton(WidgetMediaButton {
//.                                                 field: TRACKS_RECORD_KEY_FILE.to_string(),
//.                                                 media_field: FieldOrLiteral::Field(
//.                                                     TRACKS_RECORD_KEY_MEDIA.to_string(),
//.                                                 ),
//.                                                 name_field: TRACKS_RECORD_KEY_NAME.to_string(),
//.                                                 album_field: ALBUMS_RECORD_KEY_NAME.to_string(),
//.                                                 artist_field: TRACKS_RECORD_KEY_ARTIST_NAME.to_string(),
//.                                                 thumbnail_field: ALBUMS_RECORD_KEY_COVER.to_string(),
//.                                                 align: Align::Start,
//.                                             }),
//.                                             Widget::TextLine(WidgetTextLine {
//.                                                 data: FieldOrLiteral::Field(TRACKS_RECORD_KEY_INDEX.to_string()),
//.                                                 prefix: "".to_string(),
//.                                                 suffix: ".".to_string(),
//.                                                 size: "12pt".to_string(),
//.                                                 size_mode: LineSizeMode::Wrap,
//.                                                 size_max: "".to_string(),
//.                                                 orientation: Orientation::DownRight,
//.                                                 align: Align::End,
//.                                             }),
//.                                             Widget::TextLine(WidgetTextLine {
//.                                                 data: FieldOrLiteral::Field(TRACKS_RECORD_KEY_ARTIST_NAME.to_string()),
//.                                                 prefix: "".to_string(),
//.                                                 suffix: "".to_string(),
//.                                                 size: "12pt".to_string(),
//.                                                 size_mode: LineSizeMode::Wrap,
//.                                                 size_max: "5cm".to_string(),
//.                                                 orientation: Orientation::DownRight,
//.                                                 align: Align::Start,
//.                                             }),
//.                                             Widget::TextLine(WidgetTextLine {
//.                                                 data: FieldOrLiteral::Literal(" - ".to_string()),
//.                                                 prefix: "".to_string(),
//.                                                 suffix: "".to_string(),
//.                                                 size: "12pt".to_string(),
//.                                                 size_max: "".to_string(),
//.                                                 size_mode: LineSizeMode::Wrap,
//.                                                 orientation: Orientation::DownRight,
//.                                                 align: Align::Start,
//.                                             }),
//.                                             Widget::TextLine(WidgetTextLine {
//.                                                 data: FieldOrLiteral::Field(TRACKS_RECORD_KEY_NAME.to_string()),
//.                                                 prefix: "".to_string(),
//.                                                 suffix: "".to_string(),
//.                                                 size: "12pt".to_string(),
//.                                                 size_max: "5cm".to_string(),
//.                                                 size_mode: LineSizeMode::Wrap,
//.                                                 orientation: Orientation::DownRight,
//.                                                 align: Align::Start,
//.                                             })
//.                                         ],
//.                                     }),
//.                                 })
//.                             ],
//.                         })
//.                     ],
//.                 },
//.             }),
//.         },
//.     };
//. }
