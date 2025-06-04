use {
    serde::{
        Deserialize,
        Serialize,
    },
};

pub const COMIC_MANIFEST_FILENAME: &str = "sunwet.json";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ComicManifestPage {
    pub width: u32,
    pub height: u32,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ComicManifest {
    pub rtl: bool,
    pub pages: Vec<ComicManifestPage>,
}
