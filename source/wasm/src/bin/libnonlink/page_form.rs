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
    },
    flowcontrol::exenum,
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        timers::callback::Timeout,
    },
    lunk::ProcessingContext,
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
    },
    shared::interface::{
        config::form::{
            ClientForm,
            FormField,
            InputOrInline,
            InputOrInlineText,
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
    wasm::el_general::{
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
    form_id: String,
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
    return Ok(el_from_raw(input_ret.root.into()).own(|_| el_from_raw(input_ret.input.into()).on("change", {
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

pub fn build_page_form_by_id(pc: &mut ProcessingContext, form_title: &str, form_id: &str) {
    let draft_id = format!("form-draft-{}", form_id);
    set_page(form_title, el_async({
        let eg = pc.eg();
        let form_id = form_id.to_string();
        let form_title = form_title.to_string();
        async move {
            let client_config = state().client_config.get().await?;
            let Some(form) = client_config.forms.get(&form_id) else {
                return Err(format!("No form in config with id [{}]", form_id));
            };
            let error_slot =
                el_from_raw(
                    style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root.into(),
                );
            let mut out = vec![error_slot.clone()];
            let mut bar_out = vec![];
            let fs = FormState(Rc::new(FormState_ {
                form_id: form_id,
                form: form.clone(),
                data: RefCell::new(LocalStorage::get(&draft_id).unwrap_or_default()),
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
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                if let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.form_id) {
                                    v.clone()
                                } else {
                                    let v = format!("");
                                    data.insert(field.form_id.clone(), make_v(v.clone()));
                                    v
                                }
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(
                                        &id,
                                        make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value()),
                                    );
                                }
                            })),
                        );
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
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                if let Some(Node::Value(serde_json::Value::Number(v))) = data.get(&field.form_id) {
                                    v.to_string()
                                } else {
                                    let v = format!("");
                                    data.insert(field.form_id.clone(), make_v(v.clone()));
                                    v
                                }
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(
                                        &id,
                                        make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value()),
                                    );
                                }
                            })),
                        );
                    },
                    FormField::Bool(field) => {
                        fn make_v(value: bool) -> Node {
                            return Node::Value(serde_json::Value::Bool(value));
                        }

                        let input_ret = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                let initial =
                                    if let Some(Node::Value(serde_json::Value::Bool(v))) =
                                        data.get(&field.form_id) {
                                        *v
                                    } else {
                                        let v = field.initial_on;
                                        data.insert(field.form_id.clone(), make_v(v));
                                        v
                                    };
                                initial
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(
                                        &id,
                                        make_v(
                                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().checked(),
                                        ),
                                    );
                                }
                            })),
                        );
                    },
                    FormField::Date(field) => {
                        fn make_v(v: String) -> Node {
                            return Node::Value(serde_json::Value::String(v));
                        }

                        let input_ret = style_export::leaf_input_pair_date(style_export::LeafInputPairDateArgs {
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                if let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.form_id) {
                                    v.clone()
                                } else {
                                    let v = Local::now().date_naive().format("%Y-%m-%d").to_string();
                                    data.insert(field.form_id.clone(), make_v(v.clone()));
                                    v
                                }
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(&id, Node::Value(serde_json::Value::String(
                                        //. .
                                        ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                                    )));
                                }
                            })),
                        );
                    },
                    FormField::Time(field) => {
                        fn make_v(v: String) -> Node {
                            return Node::Value(serde_json::Value::String(v));
                        }

                        let input_ret = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                if let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.form_id) {
                                    v.clone()
                                } else {
                                    let v = Local::now().time().format("%H:%M:%S").to_string();
                                    data.insert(field.form_id.clone(), make_v(v.clone()));
                                    v
                                }
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(&id, Node::Value(serde_json::Value::String(
                                        //. .
                                        ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                                    )));
                                }
                            })),
                        );
                    },
                    FormField::Datetime(field) => {
                        fn make_v(v: String) -> Node {
                            return Node::Value(serde_json::Value::String(v));
                        }

                        const CHRONO_FORMAT: &str = "%Y-%m-%dT%H:%M";
                        let input_ret =
                            style_export::leaf_input_pair_datetime(style_export::LeafInputPairDatetimeArgs {
                                id: field.form_id.clone(),
                                title: field.label.clone(),
                                value: {
                                    let mut data = fs.0.data.borrow_mut();
                                    if let Some(Node::Value(serde_json::Value::String(v))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else {
                                        let v = Local::now().format(CHRONO_FORMAT).to_string();
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    }
                                },
                            });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
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
                            })),
                        );
                    },
                    FormField::Color(field) => {
                        fn make_v(v: String) -> Node {
                            return Node::Value(serde_json::Value::String(v));
                        }

                        let input_ret = style_export::leaf_input_pair_color(style_export::LeafInputPairColorArgs {
                            id: field.form_id.clone(),
                            title: field.label.clone(),
                            value: {
                                let mut data = fs.0.data.borrow_mut();
                                if let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.form_id) {
                                    v.clone()
                                } else if let Some(initial) = &field.initial {
                                    data.insert(field.form_id.clone(), make_v(initial.clone()));
                                    initial.clone()
                                } else {
                                    let v = format!("");
                                    data.insert(field.form_id.clone(), make_v(v.clone()));
                                    v
                                }
                            },
                        });
                        out.push(
                            el_from_raw(
                                input_ret.root.into(),
                            ).own(|_| el_from_raw(input_ret.input.into()).on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                                    fs.update(&id, make_v(value));
                                }
                            })),
                        );
                    },
                    FormField::ConstEnum(field) => {
                        out.push(
                            build_field_enum(
                                &fs,
                                field.form_id.clone(),
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
                                let res = req_post_json(&state().base_url, ReqQuery {
                                    query: field.query.clone(),
                                    parameters: HashMap::new(),
                                }).await?;
                                let mut choices = vec![];
                                for mut choice in res.records {
                                    let Some(value) = choice.remove("value") else {
                                        return Err(format!("Query result array element is missing `id` field"));
                                    };
                                    let TreeNode::Scalar(value) = value else {
                                        return Err(
                                            format!("Query result elements are arrays/records, not scalar values"),
                                        );
                                    };
                                    let name;
                                    if let Some(name1) = choice.remove("name") {
                                        if let TreeNode::Scalar(Node::Value(serde_json::Value::String(name1))) =
                                            name1 {
                                            name = name1;
                                        } else {
                                            name = serde_json::to_string(&name1).unwrap();
                                        }
                                    } else {
                                        name = serde_json::to_string(&value).unwrap();
                                    }
                                    choices.push((name, value));
                                }
                                return Ok(build_field_enum(&fs, field.form_id, field.label, &choices)?);
                            }
                        });
                        out.push(el_from_raw(style_export::leaf_input_pair(style_export::LeafInputPairArgs {
                            label: field.label.clone(),
                            input_id: field.form_id.clone(),
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
                let form_title = form_title.to_string();
                let error_slot = error_slot.weak();
                let fs = fs.clone();
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
                    LocalStorage::set(
                        fs.0.draft_id.clone(),
                        serde_json::to_string(&*fs.0.data.borrow()).unwrap(),
                    ).unwrap();
                    *save_thinking.borrow_mut() = Some(spawn_rooted({
                        let eg = eg.clone();
                        let fs = fs.clone();
                        let form_title = form_title.clone();
                        let error_slot = error_slot.clone();
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
                                            let Some(Node::Value(serde_json::Value::String(v))) =
                                                data.get(field) else {
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
                                req_post_json(&state().base_url, ReqCommit {
                                    add: add,
                                    remove: vec![],
                                    files: vec![],
                                }).await?;
                                return Ok(());
                            }.await {
                                Ok(_) => {
                                    LocalStorage::delete(&fs.0.draft_id);
                                    eg.event(|pc| build_page_form_by_id(pc, &form_title, &fs.0.form_id));
                                    return;
                                },
                                Err(e) => {
                                    let Some(error_slot) = error_slot.upgrade() else {
                                        return;
                                    };
                                    error_slot.ref_push(
                                        el_from_raw(
                                            style_export::leaf_err_block(style_export::LeafErrBlockArgs { data: e })
                                                .root
                                                .into(),
                                        ),
                                    );
                                    button
                                        .class_list()
                                        .remove_1(&style_export::class_state_thinking().value)
                                        .unwrap();
                                    return;
                                },
                            }
                        }
                    }));
                }
            });
            bar_out.push(button_save);
            return Ok(el_from_raw(style_export::cont_page_form(style_export::ContPageFormArgs {
                entries: out.iter().map(|x| x.raw().dyn_into::<HtmlElement>().unwrap()).collect(),
                bar_children: bar_out.iter().map(|x| x.raw().dyn_into::<HtmlElement>().unwrap()).collect(),
            }).root.into()).own(|_| (out, bar_out))) as Result<_, String>;
        }
    }));
}
