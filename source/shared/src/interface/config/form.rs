use {
    crate::interface::{
        iam::IamTargetId,
        triple::Node,
    },
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FormRelationStart {
    Subject {
        form_id: Option<String>,
    },
    Object {
        form_id: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormRelation {
    pub start: FormRelationStart,
    pub predicate: String,
    pub iam_target: IamTargetId,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldId {
    pub form_id: String,
    pub relation: Option<FormRelation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldComment {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldConstant {
    pub form_id: String,
    pub relation: FormRelation,
    value: Node,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldText {
    pub form_id: String,
    pub relation: FormRelation,
    pub label: String,
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldNumber {
    pub form_id: String,
    pub relation: FormRelation,
    pub label: String,
    pub placeholder: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FormFieldEnum {
    pub form_id: String,
    pub relation: FormRelation,
    pub label: String,
    pub choices: Vec<(String, Node)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FormField {
    Id(FormFieldId),
    Comment(FormFieldComment),
    Constant(FormFieldConstant),
    Text(FormFieldText),
    Number(FormFieldNumber),
    Enum(FormFieldEnum),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Form {
    pub allow_target: Option<IamTargetId>,
    pub id: String,
    pub name: String,
    pub fields: Vec<FormField>,
}
