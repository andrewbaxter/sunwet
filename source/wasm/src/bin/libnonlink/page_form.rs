use {
    super::{
        api::req_post_json,
        state::{
            set_page,
            state,
        },
    },
    chrono::{
        Local,
        LocalResult,
        NaiveDateTime,
        Utc,
    },
    flowcontrol::exenum,
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        timers::callback::Timeout,
    },
    lunk::{
        EventGraph,
    },
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
    },
    shared::interface::{
        config::{
            form::{
                ClientForm,
                FormField,
                InputOrInline,
                InputOrInlineText,
            },
            menu::ClientMenuItemForm,
            ClientConfig,
        },
        triple::Node,
        wire::{
            ReqCommit,
            ReqQuery,
            TreeNode,
            Triple,
        },
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm::js::{
        el_async,
        style_export,
    },
    wasm_bindgen::JsCast,
    web_sys::{
        HtmlElement,
        HtmlInputElement,
    },
};

struct FormState_ {
    draft_id: String,
    form: ClientForm,
    data: RefCell<HashMap<String, Node>>,
    draft_debounce: RefCell<Option<Timeout>>,
}

#[derive(Clone)]
struct FormState(Rc<FormState_>);

impl FormState {
    fn update(&self, field: &str, value: Node) {
        self.0.data.borrow_mut().insert(field.to_string(), value);
        *self.0.draft_debounce.borrow_mut() = Some(Timeout::new(200, {
            let s = self.clone();
            move || {
                LocalStorage::set(&s.0.draft_id, serde_json::to_string(&*s.0.data.borrow()).unwrap()).unwrap();
            }
        }));
    }
}

fn build_field_enum(
    fs: &FormState,
    field_id: String,
    field_label: String,
    choices: &Vec<(String, Node)>,
) -> Result<El, String> {
    let input_ret = style_export::leaf_input_pair_enum(style_export::LeafInputPairEnumArgs {
        id: field_id.clone(),
        title: field_label,
        value: {
            let mut data = fs.0.data.borrow_mut();
            serde_json::to_string(&match data.get(&field_id) {
                Some(x) => x.clone(),
                None => {
                    let Some((_, first)) = choices.first() else {
                        return Err(format!("Enum field {} has no choices", field_id));
                    };
                    let value = first.clone();
                    data.insert(field_id.to_string(), value.clone());
                    value
                },
            }).unwrap()
        },
        options: choices.iter().map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap())).collect(),
    });
    return Ok(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
        let id = field_id.to_string();
        let fs = fs.clone();
        move |ev| {
            let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
            fs.update(&id, if let Ok(v) = serde_json::from_str(&value) {
                v
            } else {
                Node::Value(serde_json::Value::String(value))
            });
        }
    })));
}

pub fn build_page_form(
    eg: EventGraph,
    config: ClientConfig,
    menu_item_title: String,
    menu_item: ClientMenuItemForm,
) -> Result<El, String> {
    let draft_id = format!("form-draft-{}", menu_item.id);
    let Some(form) = config.forms.get(&menu_item.form_id) else {
        return Err(format!("No form in config with id [{}]", menu_item.form_id));
    };
    let error_slot =
        el_from_raw(style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into());
    let mut out = vec![error_slot.clone()];
    let mut bar_out = vec![];
    let fs = FormState(Rc::new(FormState_ {
        form: form.clone(),
        data: RefCell::new(
            LocalStorage::get(&draft_id).unwrap_or_else(|_| form.fields.iter().filter_map(|field| match field {
                FormField::Id(field) => {
                    Some(
                        (field.id.clone(), Node::Value(serde_json::Value::String(uuid::Uuid::new_v4().to_string()))),
                    )
                },
                FormField::Comment(_field) => None,
                FormField::Text(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String("".to_string()))),
                ),
                FormField::Number(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::Number(0.into()))),
                ),
                FormField::Bool(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::Bool(field.initial_on))),
                ),
                FormField::Date(field) => Some(
                    (
                        field.id.clone(),
                        Node::Value(
                            serde_json::Value::String(Utc::now().date_naive().format("YYYY-MM-dd").to_string()),
                        ),
                    ),
                ),
                FormField::Time(field) => Some(
                    (
                        field.id.clone(),
                        Node::Value(serde_json::Value::String(Utc::now().time().format("HH:mm:ss").to_string())),
                    ),
                ),
                FormField::Datetime(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(Utc::now().to_rfc3339()))),
                ),
                FormField::RgbU8(field) => Some(
                    (
                        field.id.clone(),
                        Node::Value(
                            serde_json::Value::String(field.initial.clone().unwrap_or_else(|| format!("#000000"))),
                        ),
                    ),
                ),
                FormField::ConstEnum(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(format!("")))),
                ),
                FormField::QueryEnum(field) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(format!("")))),
                ),
            }).collect::<HashMap<_, _>>()),
        ),
        draft_debounce: RefCell::new(None),
        draft_id: draft_id,
    }));
    for field in &form.fields {
        match field {
            FormField::Id(_field) => {
                // nop
            },
            FormField::Comment(field) => {
                out.push(el("p").text(&field.text));
            },
            FormField::Text(field) => {
                fn make_v(v: String) -> Node {
                    return Node::Value(serde_json::Value::String(v));
                }

                let input_ret = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                let input = el_from_raw(input_ret.root.into());
                input.ref_own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(
                            &id,
                            make_v(
                                ev
                                    .target()
                                    .unwrap()
                                    .dyn_into::<HtmlElement>()
                                    .unwrap()
                                    .text_content()
                                    .unwrap_or_default(),
                            ),
                        );
                    }
                }));
                out.push(input);
            },
            FormField::Number(field) => {
                fn make_v(value: String) -> Node {
                    return Node::Value(if let Ok(v) = serde_json::from_str::<serde_json::Number>(&value) {
                        serde_json::Value::Number(v)
                    } else {
                        serde_json::Value::String(value)
                    });
                }

                let input_ret = style_export::leaf_input_pair_number(style_export::LeafInputPairNumberArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::Number(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.to_string()
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(
                            &id,
                            make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value()),
                        );
                    }
                })));
            },
            FormField::Bool(field) => {
                fn make_v(value: bool) -> Node {
                    return Node::Value(serde_json::Value::Bool(value));
                }

                let input_ret = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::Bool(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        *v
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(
                            &id,
                            make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().checked()),
                        );
                    }
                })));
            },
            FormField::Date(field) => {
                let input_ret = style_export::leaf_input_pair_date(style_export::LeafInputPairDateArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        )));
                    }
                })));
            },
            FormField::Time(field) => {
                let input_ret = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        )));
                    }
                })));
            },
            FormField::Datetime(field) => {
                const CHRONO_FORMAT: &str = "%Y-%m-%dT%H:%M";
                let input_ret = style_export::leaf_input_pair_datetime(style_export::LeafInputPairDatetimeArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(
                            &id,
                            Node::Value(
                                serde_json::Value::String(
                                    if let Some(v) =
                                        NaiveDateTime::parse_from_str(&value, CHRONO_FORMAT)
                                            .ok()
                                            .and_then(
                                                |v| exenum!(
                                                    v.and_local_timezone(Local),
                                                    LocalResult:: Single(v) => v.to_utc()
                                                ),
                                            ) {
                                        v.to_rfc3339()
                                    } else {
                                        value
                                    },
                                ),
                            ),
                        );
                    }
                })));
            },
            FormField::RgbU8(field) => {
                fn make_v(v: String) -> Node {
                    return Node::Value(serde_json::Value::String(v));
                }

                let input_ret = style_export::leaf_input_pair_color(style_export::LeafInputPairColorArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                out.push(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(&id, make_v(value));
                    }
                })));
            },
            FormField::ConstEnum(field) => {
                out.push(
                    build_field_enum(
                        &fs,
                        field.id.clone(),
                        field.label.clone(),
                        &field.choices.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>(),
                    )?,
                );
            },
            FormField::QueryEnum(field) => {
                let async_ = el_async({
                    let fs = fs.clone();
                    let field = field.clone();
                    async move {
                        let res = req_post_json(&state().env.base_url, ReqQuery {
                            query: field.query.clone(),
                            parameters: HashMap::new(),
                        }).await?;
                        let mut choices = vec![];
                        for mut choice in res.records {
                            let Some(value) = choice.remove("value") else {
                                return Err(format!("Query result array element is missing `id` field"));
                            };
                            let TreeNode::Scalar(value) = value else {
                                return Err(format!("Query result elements are arrays/records, not scalar values"));
                            };
                            let name;
                            if let Some(name1) = choice.remove("name") {
                                if let TreeNode::Scalar(Node::Value(serde_json::Value::String(name1))) = name1 {
                                    name = name1;
                                } else {
                                    name = serde_json::to_string(&name1).unwrap();
                                }
                            } else {
                                name = serde_json::to_string(&value).unwrap();
                            }
                            choices.push((name, value));
                        }
                        return Ok(build_field_enum(&fs, field.id, field.label, &choices)?);
                    }
                });
                out.push(el_from_raw(style_export::leaf_input_pair(style_export::LeafInputPairArgs {
                    label: field.label.clone(),
                    input_id: field.id.clone(),
                    input: async_.raw().dyn_into().unwrap(),
                }).root.into()).own(|_| async_));
            },
        }
    }
    let button_save = el_from_raw(style_export::leaf_button_big_save().root.into());
    let save_thinking = Rc::new(RefCell::new(None));
    button_save.ref_own(|_| save_thinking.clone());
    button_save.ref_on("click", {
        let eg = eg.clone();
        let error_slot = error_slot.weak();
        let fs = fs.clone();
        let menu_item_title = menu_item_title.clone();
        let config = config.clone();
        let menu_item = menu_item.clone();
        move |ev| {
            {
                let Some(error_slot) = error_slot.upgrade() else {
                    return;
                };
                error_slot.ref_clear();
            }
            let button = ev.target().unwrap().dyn_into::<HtmlElement>().unwrap();
            button.class_list().add_1(&style_export::class_state_thinking().value).unwrap();
            *fs.0.draft_debounce.borrow_mut() = None;
            LocalStorage::set(fs.0.draft_id.clone(), serde_json::to_string(&*fs.0.data.borrow()).unwrap()).unwrap();
            *save_thinking.borrow_mut() = Some(spawn_rooted({
                let eg = eg.clone();
                let fs = fs.clone();
                let error_slot = error_slot.clone();
                let menu_item_title = menu_item_title.clone();
                let config = config.clone();
                let menu_item = menu_item.clone();
                async move {
                    match async {
                        let data = fs.0.data.borrow();
                        let mut add = vec![];
                        for triple in &fs.0.form.outputs {
                            let subject;
                            match &triple.subject {
                                InputOrInline::Input(field) => {
                                    subject = data.get(field).unwrap().clone();
                                },
                                InputOrInline::Inline(v) => {
                                    subject = v.clone();
                                },
                            }
                            let predicate;
                            match &triple.predicate {
                                InputOrInlineText::Input(field) => {
                                    let Some(Node::Value(serde_json::Value::String(v))) = data.get(field) else {
                                        return Err(
                                            format!(
                                                "Field {} must be a string to be used as a predicate, but it is not",
                                                field
                                            ),
                                        );
                                    };
                                    predicate = v.clone();
                                },
                                InputOrInlineText::Inline(t) => {
                                    predicate = t.clone();
                                },
                            }
                            let object;
                            match &triple.object {
                                InputOrInline::Input(field) => {
                                    object = data.get(field).unwrap().clone();
                                },
                                InputOrInline::Inline(v) => {
                                    object = v.clone();
                                },
                            }
                            add.push(Triple {
                                subject: subject.clone(),
                                predicate: predicate.clone(),
                                object: object.clone(),
                            });
                        }
                        drop(data);
                        req_post_json(&state().env.base_url, ReqCommit {
                            add: add,
                            remove: vec![],
                            files: vec![],
                        }).await?;
                        return Ok(());
                    }.await {
                        Ok(_) => {
                            LocalStorage::delete(&fs.0.draft_id);
                            eg.event(|pc| {
                                set_page(
                                    pc,
                                    &menu_item_title,
                                    build_page_form(pc.eg(), config, menu_item_title.clone(), menu_item).unwrap(),
                                );
                            }).unwrap();
                            return;
                        },
                        Err(e) => {
                            let Some(error_slot) = error_slot.upgrade() else {
                                return;
                            };
                            error_slot.ref_push(
                                el_from_raw(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                    in_root: false,
                                    data: e,
                                }).root.into()),
                            );
                            button.class_list().remove_1(&style_export::class_state_thinking().value).unwrap();
                            return;
                        },
                    }
                }
            }));
        }
    });
    bar_out.push(button_save);
    return Ok(el_from_raw(style_export::cont_page_form(style_export::ContPageFormArgs {
        entries: out.iter().map(|x| x.raw()).collect(),
        bar_children: bar_out.iter().map(|x| x.raw()).collect(),
    }).root.into()).own(|_| (out, bar_out))) as Result<_, String>;
}
