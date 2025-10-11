use {
    crate::interface::{
        triple::Node,
        wire::TreeNode,
    },
    std::collections::BTreeMap,
};

pub fn node_to_text(node: &Node) -> String {
    match node {
        Node::File(node) => return node.to_string(),
        Node::Value(node) => match node {
            serde_json::Value::String(v) => return v.clone(),
            serde_json::Value::Number(v) => if v.is_u64() {
                return v.as_u64().unwrap().to_string();
            } else if v.is_i64() {
                return v.as_i64().unwrap().to_string();
            } else if v.is_f64() {
                return v.as_f64().unwrap().to_string();
            } else {
                unreachable!();
            },
            node => return serde_json::to_string(node).unwrap(),
        },
    };
}

pub enum PatternPart {
    Lit(String),
    Field(String),
}

pub struct Pattern {
    pub parts: Vec<PatternPart>,
}

impl Pattern {
    pub fn interpolate(&self, args: &BTreeMap<String, TreeNode>) -> String {
        return self.parts.iter().map(|x| match x {
            PatternPart::Lit(s) => s.clone(),
            PatternPart::Field(s) => match args.get(s) {
                Some(TreeNode::Scalar(s)) => node_to_text(&s),
                Some(s) => serde_json::to_string(s).unwrap(),
                None => format!(""),
            },
        }).collect::<Vec<_>>().join("");
    }
}

impl<'a> From<&'a str> for Pattern {
    fn from(pattern: &'a str) -> Self {
        let mut field = false;
        let mut escape = false;
        let mut buf = vec![];
        let mut parts = vec![];
        for c in pattern.chars() {
            if escape {
                escape = false;
                buf.push(c);
            } else if c == '\\' {
                escape = true;
            } else {
                if field {
                    if c == '}' {
                        field = false;
                        if !buf.is_empty() {
                            parts.push(PatternPart::Field(buf.drain(..).collect()));
                        }
                    } else {
                        buf.push(c);
                    }
                } else {
                    if c == '{' {
                        field = true;
                        if !buf.is_empty() {
                            parts.push(PatternPart::Lit(buf.drain(..).collect()));
                        }
                    } else {
                        buf.push(c);
                    }
                }
            }
        }
        if !buf.is_empty() {
            if field {
                parts.push(PatternPart::Field(buf.drain(..).collect()));
            } else {
                parts.push(PatternPart::Lit(buf.drain(..).collect()));
            }
        }
        return Self { parts: parts };
    }
}

impl ToString for Pattern {
    fn to_string(&self) -> String {
        return self.parts.iter().map(|x| match x {
            PatternPart::Lit(p) => p.clone(),
            PatternPart::Field(p) => format!("{{{}}}", p.replace("{", "\\{").replace("}", "\\}")),
        }).collect::<Vec<_>>().join("");
    }
}
