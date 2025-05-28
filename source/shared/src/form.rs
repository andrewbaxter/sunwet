use {
    crate::interface::{
        config::form::{
            FormOutput,
            InputOrInline,
            InputOrInlineText,
        },
        triple::Node,
        wire::{
            ReqCommit,
            TreeNode,
            Triple,
        },
    },
    std::collections::HashMap,
};

pub fn build_form_commit(
    outputs: &Vec<FormOutput>,
    params: &HashMap<String, TreeNode>,
) -> Result<ReqCommit, String> {
    let mut add = vec![];
    let get_data = |field| {
        let v = params.get(field).unwrap();
        match v {
            TreeNode::Scalar(v) => {
                return Ok(vec![v.clone()]);
            },
            TreeNode::Array(ns) => {
                let mut s1 = vec![];
                for v in ns {
                    let TreeNode::Scalar(v) = v else {
                        return Err(format!("Nested QueryResValue field in form data (likely bug)"));
                    };
                    s1.push(v.clone());
                }
                return Ok(s1);
            },
            TreeNode::Record(_) => {
                return Err(format!("Record QueryResValue field in form data (likely bug)"));
            },
        }
    };
    for triple in outputs {
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
                let Some(TreeNode::Scalar(Node::Value(serde_json::Value::String(v)))) = params.get(field) else {
                    return Err(format!("Field {} must be a string to be used as a predicate, but it is not", field));
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
    return Ok(ReqCommit {
        add: add,
        remove: vec![],
        files: vec![],
    });
}
