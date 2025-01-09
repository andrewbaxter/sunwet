use {
    super::state::State,
    crate::{
        el_general::{
            el_async,
            el_button_icon_text,
            el_buttonbox,
            el_err_block,
            el_err_span,
            CSS_FORM_SECTION,
            CSS_S_FORM,
            ICON_SAVE,
        },
        state::set_page,
        world::{
            self,
            req_post_json,
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
        spawn_rooted,
        El,
    },
    shared::interface::{
        config::form::{
            Form,
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
    wasm_bindgen::JsCast,
    web_sys::HtmlInputElement,
};

struct FormState_ {
    draft_id: String,
    form: Form,
    data: RefCell<HashMap<String, TreeNode>>,
    draft_debounce: RefCell<Option<Timeout>>,
}

#[derive(Clone)]
struct FormState(Rc<FormState_>);

impl FormState {
    fn update(&self, field: &str, value: TreeNode) {
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
    field_id: &str,
    field_label: &str,
    choices: &Vec<(String, TreeNode)>,
) -> Result<El, String> {
    let input = el("select").on("change", {
        let id = field_id.to_string();
        let fs = fs.clone();
        move |ev| {
            let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
            fs.update(&id, if let Ok(v) = serde_json::from_str(&value) {
                v
            } else {
                TreeNode::Scalar(Node::Value(serde_json::Value::String(value)))
            });
        }
    });
    let draft_value = {
        let mut data = fs.0.data.borrow_mut();
        match data.get(field_id) {
            Some(x) => x.clone(),
            None => {
                let Some((_, first)) = choices.first() else {
                    return Err(format!("Enum field {} has no choices", field_id));
                };
                let value = first.clone();
                data.insert(field_id.to_string(), value.clone());
                value
            },
        }
    };
    for choice in choices {
        let value = choice.1.clone();
        let option = el("option").attr("value", &serde_json::to_string(&value).unwrap());
        if draft_value == value {
            option.ref_attr("selected", "selected");
        }
        input.ref_push(option.text(&choice.0));
    }
    return Ok(el("label").push(el("span").text(field_label)).push(input.clone()));
}

pub fn build_page_form_by_id(pc: &mut ProcessingContext, outer_state: &State, form_title: &str, form_id: &str) {
    let draft_id = format!("form-draft-{}", form_id);
    set_page(outer_state, form_title, el_async().own(|async_el| {
        let async_el = async_el.weak();
        let eg = pc.eg();
        let outer_state = outer_state.clone();
        let form_id = form_id.to_string();
        let form_title = form_title.to_string();
        spawn_rooted(async move {
            let menu = match outer_state.menu.get().await {
                Ok(m) => m,
                Err(e) => {
                    let Some(async_el) = async_el.upgrade() else {
                        return;
                    };
                    async_el.ref_replace(vec![el_err_block(e)]);
                    return;
                },
            };
            let Some(async_el) = async_el.upgrade() else {
                return;
            };
            eg.event(|pc| {
                let Some(form) = menu.forms.get(&form_id) else {
                    async_el.ref_replace(vec![el_err_block("Unknown form".to_string())]);
                    return;
                };
                let mut out = vec![];
                let fs = FormState(Rc::new(FormState_ {
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
                            fn make_v(v: String) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::String(v)));
                            }

                            let input =
                                el("div")
                                    .classes(&["input"])
                                    .attr("contenteditable", "plaintextonly")
                                    .on("change", {
                                        let id = field.form_id.clone();
                                        let fs = fs.clone();
                                        move |ev| {
                                            fs.update(
                                                &id,
                                                make_v(
                                                    ev
                                                        .target()
                                                        .unwrap()
                                                        .dyn_into::<HtmlInputElement>()
                                                        .unwrap()
                                                        .value(),
                                                ),
                                            );
                                        }
                                    });
                            if let Some(placeholder) = &field.placeholder {
                                input.ref_attr("placeholder", &placeholder);
                            }
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Number(field) => {
                            fn make_v(value: String) -> TreeNode {
                                return TreeNode::Scalar(
                                    Node::Value(if let Ok(v) = serde_json::from_str::<serde_json::Number>(&value) {
                                        serde_json::Value::Number(v)
                                    } else {
                                        serde_json::Value::String(value)
                                    }),
                                );
                            }

                            let input = el("input").attr("type", "number").on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    fs.update(
                                        &id,
                                        make_v(ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value()),
                                    );
                                }
                            });
                            if let Some(placeholder) = &field.placeholder {
                                input.ref_attr("placeholder", &placeholder);
                            }
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::Number(v)))) =
                                        data.get(&field.form_id) {
                                        v.to_string()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Bool(field) => {
                            fn make_v(value: bool) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::Bool(value)));
                            }

                            let input = el("input").attr("type", "checkbox").on("change", {
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
                            });
                            let initial;
                            {
                                let mut data = fs.0.data.borrow_mut();
                                initial =
                                    if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::Bool(v)))) =
                                        data.get(&field.form_id) {
                                        *v
                                    } else {
                                        let v = field.initial_on;
                                        data.insert(field.form_id.clone(), make_v(v));
                                        v
                                    };
                            }
                            if initial {
                                input.ref_attr("checked", "checked");
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Date(field) => {
                            fn make_v(v: String) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::String(v)));
                            }

                            let input =
                                el("input")
                                    .attr("type", "date")
                                    .attr("value", &Local::now().date_naive().format("%Y-%m-%d").to_string())
                                    .on("change", {
                                        let id = field.form_id.clone();
                                        let fs = fs.clone();
                                        move |ev| {
                                            fs.update(&id, TreeNode::Scalar(Node::Value(serde_json::Value::String(
                                                //. .
                                                ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                                            ))));
                                        }
                                    });
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Time(field) => {
                            fn make_v(v: String) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::String(v)));
                            }

                            let input =
                                el("input")
                                    .attr("type", "time")
                                    .attr("value", &Local::now().time().format("%H:%M:%S").to_string())
                                    .on("change", {
                                        let id = field.form_id.clone();
                                        let fs = fs.clone();
                                        move |ev| {
                                            fs.update(&id, TreeNode::Scalar(Node::Value(serde_json::Value::String(
                                                //. .
                                                ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value(),
                                            ))));
                                        }
                                    });
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Datetime(field) => {
                            fn make_v(v: String) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::String(v)));
                            }

                            const CHRONO_FORMAT: &str = "%Y-%m-%dT%H:%M";
                            let input =
                                el("input")
                                    .attr("type", "datetime-local")
                                    .attr("value", &Local::now().format(CHRONO_FORMAT).to_string())
                                    .on("change", {
                                        let id = field.form_id.clone();
                                        let fs = fs.clone();
                                        move |ev| {
                                            let value =
                                                ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                                            fs.update(
                                                &id,
                                                TreeNode::Scalar(
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
                                                ),
                                            );
                                        }
                                    });
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::Color(field) => {
                            fn make_v(v: String) -> TreeNode {
                                return TreeNode::Scalar(Node::Value(serde_json::Value::String(v)));
                            }

                            let input = el("input").attr("type", "color").on("change", {
                                let id = field.form_id.clone();
                                let fs = fs.clone();
                                move |ev| {
                                    let value = ev.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value();
                                    fs.update(&id, make_v(value));
                                }
                            });
                            {
                                let mut data = fs.0.data.borrow_mut();
                                input.ref_attr(
                                    "value",
                                    &if let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) =
                                        data.get(&field.form_id) {
                                        v.clone()
                                    } else if let Some(initial) = &field.initial {
                                        data.insert(field.form_id.clone(), make_v(initial.clone()));
                                        initial.clone()
                                    } else {
                                        let v = format!("");
                                        data.insert(field.form_id.clone(), make_v(v.clone()));
                                        v
                                    },
                                );
                            }
                            out.push(el("label").push(el("span").text(&field.label)).push(input.clone()));
                        },
                        FormField::ConstEnum(field) => {
                            let choices =
                                field
                                    .choices
                                    .iter()
                                    .map(|(k, v)| (k.clone(), TreeNode::Scalar(v.clone())))
                                    .collect::<Vec<_>>();
                            out.push(match build_field_enum(&fs, &field.form_id, &field.label, &choices) {
                                Ok(e) => e,
                                Err(e) => el_err_span(e),
                            });
                        },
                        FormField::QueryEnum(field) => {
                            out.push(
                                el("label")
                                    .push(el("span").text(&field.label))
                                    .push(el_async().own(|async_el| spawn_rooted({
                                        let fs = fs.clone();
                                        let outer_state = outer_state.clone();
                                        let async_el = async_el.weak();
                                        let field = field.clone();
                                        async move {
                                            match async {
                                                let res = req_post_json(&outer_state.base_url, ReqQuery {
                                                    query: field.query.clone(),
                                                    parameters: HashMap::new(),
                                                }).await?;
                                                let TreeNode::Array(res) = res.records else {
                                                    return Err(
                                                        format!("Result is not an array of choices (likely bug)"),
                                                    );
                                                };
                                                let mut choices = vec![];
                                                for choice in res {
                                                    let TreeNode::Record(mut choice) = choice else {
                                                        return Err(
                                                            format!(
                                                                "Query result array element is not a record (likely bug)"
                                                            ),
                                                        );
                                                    };
                                                    let Some(value) = choice.remove("value") else {
                                                        return Err(
                                                            format!("Query result array element is missing `id` field"),
                                                        );
                                                    };
                                                    let name;
                                                    if let Some(name1) = choice.remove("name") {
                                                        if let TreeNode::Scalar(
                                                            Node::Value(serde_json::Value::String(name1)),
                                                        ) =
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
                                                return Ok(
                                                    build_field_enum(&fs, &field.form_id, &field.label, &choices)?,
                                                );
                                            }.await {
                                                Ok(r) => {
                                                    let Some(async_el) = async_el.upgrade() else {
                                                        return;
                                                    };
                                                    async_el.ref_replace(vec![r]);
                                                },
                                                Err(e) => {
                                                    let Some(async_el) = async_el.upgrade() else {
                                                        return;
                                                    };
                                                    async_el.ref_replace(vec![el_err_span(e)]);
                                                },
                                            }
                                        }
                                    }))),
                            );
                        },
                    }
                }
                out.push(el_buttonbox().push(el_button_icon_text(pc, ICON_SAVE, "Save", {
                    let outer_state = outer_state.clone();
                    let form_title = form_title.to_string();
                    let fs = fs.clone();
                    move |pc| {
                        *fs.0.draft_debounce.borrow_mut() = None;
                        LocalStorage::set(
                            fs.0.draft_id.clone(),
                            serde_json::to_string(&*fs.0.data.borrow()).unwrap(),
                        ).unwrap();
                        let root = outer_state.page_body.upgrade().unwrap();
                        root.ref_clear();
                        root.ref_push(el_async().own(|async_el| {
                            let fs = fs.clone();
                            let outer_state = outer_state.clone();
                            let async_el = async_el.weak();
                            let form_title = form_title.clone();
                            let eg = pc.eg();
                            spawn_rooted(async move {
                                match async {
                                    let data = fs.0.data.borrow();
                                    let mut add = vec![];
                                    let get_data = |field| {
                                        let v = data.get(field).unwrap();
                                        match v {
                                            TreeNode::Scalar(v) => {
                                                return Ok(vec![v.clone()]);
                                            },
                                            TreeNode::Array(ns) => {
                                                let mut s1 = vec![];
                                                for v in ns {
                                                    let TreeNode::Scalar(v) = v else {
                                                        return Err(
                                                            format!(
                                                                "Nested TreeNodeue field in form data (likely bug)"
                                                            ),
                                                        );
                                                    };
                                                    s1.push(v.clone());
                                                }
                                                return Ok(s1);
                                            },
                                            TreeNode::Record(_) => {
                                                return Err(
                                                    format!("Record TreeNodeue field in form data (likely bug)"),
                                                );
                                            },
                                        }
                                    };
                                    for triple in &fs.0.form.outputs {
                                        let subjects;
                                        match &triple.subject {
                                            InputOrInline::Input(field) => {
                                                subjects = get_data(field)?;
                                            },
                                            InputOrInline::Inline(v) => {
                                                subjects = vec![v.clone()];
                                            },
                                        }
                                        let predicate;
                                        match &triple.predicate {
                                            InputOrInlineText::Input(field) => {
                                                let Some(
                                                    TreeNode::Scalar(Node::Value(serde_json::Value::String(v))),
                                                ) =
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
                                        let objects;
                                        match &triple.object {
                                            InputOrInline::Input(field) => {
                                                objects = get_data(field)?;
                                            },
                                            InputOrInline::Inline(v) => {
                                                objects = vec![v.clone()];
                                            },
                                        }
                                        for subj in subjects {
                                            for obj in &objects {
                                                add.push(Triple {
                                                    subject: subj.clone(),
                                                    predicate: predicate.clone(),
                                                    object: obj.clone(),
                                                });
                                            }
                                        }
                                    }
                                    drop(data);
                                    world::req_post_json(&outer_state.base_url, ReqCommit {
                                        add: add,
                                        remove: vec![],
                                        files: vec![],
                                    }).await?;
                                    return Ok(());
                                }.await {
                                    Ok(_) => {
                                        LocalStorage::delete(&fs.0.draft_id);
                                        eg.event(
                                            |pc| build_page_form_by_id(pc, &outer_state, &form_title, &fs.0.form.id),
                                        );
                                        return;
                                    },
                                    Err(e) => {
                                        let Some(async_el) = async_el.upgrade() else {
                                            return;
                                        };
                                        async_el.ref_replace(vec![el_err_block(e)]);
                                        return;
                                    },
                                }
                            })
                        }));
                    }
                })));
                async_el.ref_replace(
                    vec![
                        el("div")
                            .classes(&[CSS_S_FORM])
                            .push(el("div").classes(&[CSS_FORM_SECTION]).extend(out))
                    ],
                );
            });
        })
    }));
}
