use {
    super::{
        api::req_post_json,
        commit::{
            prep_node,
            upload_files,
            CommitNode,
        },
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
    lunk::EventGraph,
    rooting::{
        el,
        spawn_rooted,
        El,
    },
    shared::{
        interface::{
            config::{
                form::{
                    ClientForm,
                    FormFieldType,
                },
                ClientConfig,
            },
            triple::Node,
            wire::{
                ReqFormCommit,
                ReqQuery,
                TreeNode,
            },
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
    data: RefCell<HashMap<String, CommitNode>>,
    draft_debounce: RefCell<Option<Timeout>>,
}

#[derive(Clone)]
struct FormState(Rc<FormState_>);

impl FormState {
    fn update(&self, field: &str, value: CommitNode) {
        self.0.data.borrow_mut().insert(field.to_string(), value);
        *self.0.draft_debounce.borrow_mut() = Some(Timeout::new(200, {
            let s = self.clone();
            move || {
                LocalStorage::set(
                    &s.0.draft_id,
                    serde_json::to_string(&s.0.data.borrow().iter().filter_map(|(k, v)| match v {
                        CommitNode::Node(v) => {
                            return Some((k, v));
                        },
                        CommitNode::File(..) => {
                            return None;
                        },
                    }).collect::<HashMap<_, _>>()).unwrap(),
                ).unwrap();
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
            let node = match data.get(&field_id) {
                Some(x) => x.clone(),
                None => {
                    let Some((_, first)) = choices.first() else {
                        return Err(format!("Enum field {} has no choices", field_id));
                    };
                    let value = CommitNode::Node(first.clone());
                    data.insert(field_id.to_string(), value.clone());
                    value
                },
            };
            serde_json::to_string(&match &node {
                CommitNode::Node(node) => node,
                CommitNode::File(..) => &choices[0].1,
            }).unwrap()
        },
        options: choices.iter().map(|(k, v)| (k.clone(), serde_json::to_string(v).unwrap())).collect(),
    });
    input_ret.input.ref_on("input", {
        let id = field_id.to_string();
        let fs = fs.clone();
        move |ev| {
            let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
            fs.update(&id, if let Ok(v) = serde_json::from_str(&value) {
                CommitNode::Node(v)
            } else {
                CommitNode::Node(Node::Value(serde_json::Value::String(value)))
            });
        }
    });
    return Ok(input_ret.root);
}

pub fn build_page_form(
    eg: EventGraph,
    config: ClientConfig,
    menu_item_title: String,
    form: ClientForm,
) -> Result<El, String> {
    let draft_id = format!("form-draft-{}", form.id);
    let error_slot = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;
    let mut out = vec![error_slot.clone()];
    let mut bar_out = vec![];
    let fs = FormState(Rc::new(FormState_ {
        data: RefCell::new(
            LocalStorage::get::<HashMap<String, Node>>(&draft_id)
                .map(|m| m.into_iter().map(|(k, v)| (k, CommitNode::Node(v))).collect::<HashMap<_, _>>())
                .unwrap_or_else(|_| Default::default()),
        ),
        draft_debounce: Default::default(),
        draft_id: draft_id,
    }));
    for (index, field) in form.fields.iter().enumerate() {
        match &field.r#type {
            FormFieldType::Id => {
                // nop
            },
            FormFieldType::Comment(field2) => {
                out.push(el("p").text(&field2.text));
            },
            FormFieldType::Text(_field2) => {
                let input_ret = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::String(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.clone()
                    },
                });
                let input = input_ret.root;
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlElement>().unwrap().text_content().unwrap_or_default(),
                        ))));
                    }
                });
                out.push(input);
            },
            FormFieldType::Number(_field2) => {
                let input_ret = style_export::leaf_input_pair_number(style_export::LeafInputPairNumberArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::Number(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.to_string()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(
                            &id,
                            CommitNode::Node(
                                Node::Value(if let Ok(v) = serde_json::from_str::<serde_json::Number>(&value) {
                                    serde_json::Value::Number(v)
                                } else {
                                    serde_json::Value::String(value)
                                }),
                            ),
                        );
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::Bool(_field2) => {
                let input_ret = style_export::leaf_input_pair_bool(style_export::LeafInputPairBoolArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::Bool(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        *v
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().checked();
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::Bool(value))));
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::Date => {
                let input_ret = style_export::leaf_input_pair_date(style_export::LeafInputPairDateArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::String(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.clone()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        ))));
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::Time => {
                let input_ret = style_export::leaf_input_pair_text(style_export::LeafInputPairTextArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::String(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.clone()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        ))));
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::Datetime => {
                const CHRONO_FORMAT: &str = "%Y-%m-%dT%H:%M";
                let input_ret = style_export::leaf_input_pair_datetime(style_export::LeafInputPairDatetimeArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::String(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.clone()
                    },
                });
                input_ret.input.on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::String(
                            //. .
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
                        ))));
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::RgbU8(_field2) => {
                let input_ret = style_export::leaf_input_pair_color(style_export::LeafInputPairColorArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                    value: {
                        let data = fs.0.data.borrow_mut();
                        let Some(CommitNode::Node(Node::Value(serde_json::Value::String(v)))) =
                            data.get(&field.id) else {
                                panic!();
                            };
                        v.clone()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(&id, CommitNode::Node(Node::Value(serde_json::Value::String(value))));
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::ConstEnum(field2) => {
                out.push(
                    build_field_enum(
                        &fs,
                        field.id.clone(),
                        field.label.clone(),
                        &field2.choices.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>(),
                    )?,
                );
            },
            FormFieldType::QueryEnum(field2) => {
                let async_ = el_async({
                    let fs = fs.clone();
                    let id = field.id.clone();
                    let label = field.label.clone();
                    let field2 = field2.clone();
                    async move {
                        let res = req_post_json(&state().env.base_url, ReqQuery {
                            query: field2.query.clone(),
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
                        return Ok(vec![build_field_enum(&fs, id.clone(), label.clone(), &choices)?]);
                    }
                });
                out.push(style_export::leaf_input_pair(style_export::LeafInputPairArgs {
                    label: field.label.clone(),
                    input_id: field.id.clone(),
                    input: async_,
                }).root);
            },
            FormFieldType::File => {
                let style_res = style_export::leaf_input_pair_file(style_export::LeafInputPairFileArgs {
                    id: field.id.clone(),
                    title: field.label.clone(),
                });
                style_res.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let file =
                            ev
                                .target()
                                .unwrap()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap()
                                .files()
                                .unwrap()
                                .item(0)
                                .unwrap();
                        fs.update(&id, CommitNode::File(index, file));
                    }
                });
                out.push(style_res.root);
            },
        }
    }
    let button_save = style_export::leaf_button_big_save().root;
    let save_thinking = Rc::new(RefCell::new(None));
    button_save.ref_own(|_| save_thinking.clone());
    button_save.ref_on("click", {
        let eg = eg.clone();
        let error_slot = error_slot.weak();
        let fs = fs.clone();
        let menu_item_title = menu_item_title.clone();
        let config = config.clone();
        let menu_item = form.clone();
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
            *save_thinking.borrow_mut() = Some(spawn_rooted({
                let eg = eg.clone();
                let fs = fs.clone();
                let error_slot = error_slot.clone();
                let menu_item_title = menu_item_title.clone();
                let config = config.clone();
                let menu_item = menu_item.clone();
                async move {
                    match async {
                        let data = fs.0.data.borrow().clone();
                        let mut params_to_post = HashMap::new();
                        let mut files_to_return = HashMap::new();
                        let mut files_to_commit = vec![];
                        let mut files_to_upload = vec![];
                        for (k, v) in data {
                            let Some(n) =
                                prep_node(
                                    &mut files_to_return,
                                    &mut files_to_commit,
                                    &mut files_to_upload,
                                    v,
                                ).await else {
                                    continue;
                                };
                            params_to_post.insert(k.clone(), TreeNode::Scalar(n));
                        }
                        req_post_json(&state().env.base_url, ReqFormCommit {
                            menu_item_id: menu_item.id.clone(),
                            parameters: params_to_post,
                        }).await?;
                        upload_files(files_to_upload).await?;
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
                            error_slot.ref_push(style_export::leaf_err_block(style_export::LeafErrBlockArgs {
                                in_root: false,
                                data: e,
                            }).root);
                            button.class_list().remove_1(&style_export::class_state_thinking().value).unwrap();
                            return;
                        },
                    }
                }
            }));
        }
    });
    bar_out.push(button_save);
    return Ok(style_export::cont_page_form(style_export::ContPageFormArgs {
        entries: out,
        bar_children: bar_out,
    }).root) as Result<_, String>;
}
