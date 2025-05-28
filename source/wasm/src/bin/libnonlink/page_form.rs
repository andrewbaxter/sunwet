use {
    super::{
        api::req_post_json,
        state::{
            set_page,
            state,
        },
    },
    crate::libnonlink::api::file_post_json,
    chrono::{
        Local,
        LocalResult,
        NaiveDateTime,
        Utc,
    },
    flowcontrol::{
        exenum,
        shed,
    },
    gloo::{
        file::{
            callbacks::{
                self,
                FileReader,
            },
            Blob,
        },
        storage::{
            LocalStorage,
            Storage,
        },
        timers::{
            callback::Timeout,
            future::TimeoutFuture,
        },
        utils::window,
    },
    lunk::EventGraph,
    rooting::{
        el,
        el_from_raw,
        spawn_rooted,
        El,
    },
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::{
        config::{
            form::{
                ClientForm,
                FormField,
                FormFieldType,
                InputOrInline,
                InputOrInlineText,
            },
            ClientConfig,
        },
        triple::{
            FileHash,
            Node,
        },
        wire::{
            CommitFile,
            ReqCommit,
            ReqQuery,
            ReqUploadFinish,
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
        log,
        log_js,
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
    data_files: RefCell<HashMap<String, web_sys::File>>,
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
    input_ret.input.ref_on("input", {
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
        form: form.clone(),
        data: RefCell::new(
            LocalStorage::get(
                &draft_id,
            ).unwrap_or_else(|_| form.fields.iter().filter_map(|field| match &field.r#type {
                FormFieldType::Id => {
                    Some(
                        (field.id.clone(), Node::Value(serde_json::Value::String(uuid::Uuid::new_v4().to_string()))),
                    )
                },
                FormFieldType::Comment(_field2) => None,
                FormFieldType::Text(_field2) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String("".to_string()))),
                ),
                FormFieldType::Number(_field2) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::Number(0.into()))),
                ),
                FormFieldType::Bool(field2) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::Bool(field2.initial_on))),
                ),
                FormFieldType::Date => Some(
                    (
                        field.id.clone(),
                        Node::Value(
                            serde_json::Value::String(Utc::now().date_naive().format("YYYY-MM-dd").to_string()),
                        ),
                    ),
                ),
                FormFieldType::Time => Some(
                    (
                        field.id.clone(),
                        Node::Value(serde_json::Value::String(Utc::now().time().format("HH:mm:ss").to_string())),
                    ),
                ),
                FormFieldType::Datetime => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(Utc::now().to_rfc3339()))),
                ),
                FormFieldType::RgbU8(field2) => Some(
                    (
                        field.id.clone(),
                        Node::Value(
                            serde_json::Value::String(field2.initial.clone().unwrap_or_else(|| format!("#000000"))),
                        ),
                    ),
                ),
                FormFieldType::ConstEnum(_field2) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(format!("")))),
                ),
                FormFieldType::QueryEnum(_field2) => Some(
                    (field.id.clone(), Node::Value(serde_json::Value::String(format!("")))),
                ),
                FormFieldType::File(_) => None,
            }).collect::<HashMap<_, _>>()),
        ),
        data_files: Default::default(),
        draft_debounce: Default::default(),
        draft_id: draft_id,
    }));
    for field in &form.fields {
        match &field.r#type {
            FormFieldType::Id => {
                // nop
            },
            FormFieldType::Comment(field2) => {
                out.push(el("p").text(&field2.text));
            },
            FormFieldType::Text(field2) => {
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
                let input = input_ret.root;
                input_ret.input.ref_on("input", {
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
                });
                out.push(input);
            },
            FormFieldType::Number(field2) => {
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
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(
                            &id,
                            make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value()),
                        );
                    }
                });
                out.push(input_ret.root);
            },
            FormFieldType::Bool(field2) => {
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
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(
                            &id,
                            make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().checked()),
                        );
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
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        )));
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
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
                            panic!();
                        };
                        v.clone()
                    },
                });
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        fs.update(&id, Node::Value(serde_json::Value::String(
                            //. .
                            ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                        )));
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
                        let Some(Node::Value(serde_json::Value::String(v))) = data.get(&field.id) else {
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
                });
                out.push(input_ret.root);
            },
            FormFieldType::RgbU8(_field2) => {
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
                input_ret.input.ref_on("input", {
                    let id = field.id.clone();
                    let fs = fs.clone();
                    move |ev| {
                        let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                        fs.update(&id, make_v(value));
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
                        return Ok(build_field_enum(&fs, id.clone(), label.clone(), &choices)?);
                    }
                });
                out.push(style_export::leaf_input_pair(style_export::LeafInputPairArgs {
                    label: field.label.clone(),
                    input_id: field.id.clone(),
                    input: async_,
                }).root);
            },
            FormFieldType::File(field2) => {
                let root;
                let file_input;
                match field2.r#type {
                    shared::interface::config::form::FormFieldFileType::Any => {
                        let style_res = style_export::leaf_input_pair_file(style_export::LeafInputPairFileArgs {
                            id: field.id.clone(),
                            title: field.label.clone(),
                        });
                        root = style_res.root;
                        file_input = style_res.input;
                    },
                    shared::interface::config::form::FormFieldFileType::Image => {
                        let style_res = style_export::leaf_input_pair_image(style_export::LeafInputPairImageArgs {
                            id: field.id.clone(),
                            title: field.label.clone(),
                        });
                        root = style_res.root;
                        file_input = style_res.input;
                    },
                    shared::interface::config::form::FormFieldFileType::Video => {
                        let style_res = style_export::leaf_input_pair_video(style_export::LeafInputPairVideoArgs {
                            id: field.id.clone(),
                            title: field.label.clone(),
                        });
                        root = style_res.root;
                        file_input = style_res.input;
                    },
                    shared::interface::config::form::FormFieldFileType::Audio => {
                        let style_res = style_export::leaf_input_pair_audio(style_export::LeafInputPairAudioArgs {
                            id: field.id.clone(),
                            title: field.label.clone(),
                        });
                        root = style_res.root;
                        file_input = style_res.input;
                    },
                }
                file_input.ref_on("input", {
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
                        fs.0.data_files.borrow_mut().insert(id.clone(), file.clone());
                    }
                });
                out.push(root);
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
                        // Hash files
                        struct UploadFile {
                            data: Vec<u8>,
                            hash: FileHash,
                            size: u64,
                        }

                        let mut commit_files = vec![];
                        let mut upload_files = vec![];
                        for (id, file) in fs.0.data_files.borrow().iter() {
                            let b = match gloo::file::futures::read_as_bytes(&Blob::from(file.clone())).await {
                                Ok(b) => b,
                                Err(e) => {
                                    log(format!("Error reading file for field [{}]: {}", id, e));
                                    continue;
                                },
                            };
                            let hash = FileHash::from_sha256(Sha256::digest(&b));
                            fs.0.data.borrow_mut().insert(id.clone(), Node::File(hash.clone()));
                            let size = file.size() as u64;
                            upload_files.push(UploadFile {
                                data: b,
                                hash: hash.clone(),
                                size: size,
                            });
                            commit_files.push(CommitFile {
                                hash: hash,
                                size: size,
                                mimetype: file.type_(),
                            })
                        }

                        // Send commit
                        let data = fs.0.data.borrow();
                        let mut add = vec![];
                        for triple in &fs.0.form.outputs {
                            let subject;
                            match &triple.subject {
                                InputOrInline::Input(field) => {
                                    let Some(s1) = data.get(field) else {
                                        continue;
                                    };
                                    subject = s1.clone();
                                },
                                InputOrInline::Inline(v) => {
                                    subject = v.clone();
                                },
                            }
                            let predicate;
                            match &triple.predicate {
                                InputOrInlineText::Input(field) => {
                                    let Some(p1) = data.get(field) else {
                                        continue;
                                    };
                                    let Node::Value(serde_json::Value::String(v)) = p1 else {
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
                                    let Some(o1) = data.get(field) else {
                                        continue;
                                    };
                                    object = o1.clone();
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
                            files: commit_files,
                        }).await?;

                        // Upload files
                        for file in upload_files {
                            const CHUNK_SIZE: u64 = 1024 * 1024 * 8;
                            let chunks = file.size.div_ceil(CHUNK_SIZE);
                            for i in 0 .. chunks {
                                let chunk_start = i * CHUNK_SIZE;
                                let chunk_size = (file.size - chunk_start).min(CHUNK_SIZE);
                                file_post_json(
                                    &state().env.base_url,
                                    &file.hash,
                                    chunk_start,
                                    &file.data[chunk_start as usize .. (chunk_start + chunk_size) as usize],
                                ).await?;
                            }
                            loop {
                                let resp =
                                    req_post_json(&state().env.base_url, ReqUploadFinish(file.hash.clone())).await?;
                                if resp.done {
                                    break;
                                }
                                TimeoutFuture::new(1000).await;
                            }
                        }
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
