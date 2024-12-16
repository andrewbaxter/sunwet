use {
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityPublic {
    pub admin_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityOidc {
    pub admin_token: Option<String>,
    pub oid_provider_url: String,
    pub oid_client_id: String,
    pub oid_client_secret: Option<String>,
    pub fdap_url: String,
    pub fdap_token: String,
    /// Path below default user path for sunwet-specific data
    pub fdap_user_subpath: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Identity {
    /// No identification, all users are admin. Use this when you have your own portal
    /// and don't need to support multiple users.
    Admin,
    /// No identification, all users are "world". Admin has a token for API requests.
    Public(IdentityPublic),
    /// Identification is done via OIDC, with user configs fetched from FDAP directly.
    Oidc(IdentityOidc),
}
