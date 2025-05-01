use {
    shared::interface::{
        triple::FileHash,
        wire::{
            FileGenerated,
            FileUrlQuery,
        },
    },
};

pub fn generated_file_url(file_url: &str, generation: &str, mime: &str) -> String {
    return format!("{}?{}", file_url, serde_json::to_string(&FileUrlQuery { generated: Some(FileGenerated {
        name: generation.to_string(),
        mime_type: mime.to_string(),
    }) }).unwrap());
}

pub fn file_url(base_url: &String, hash: &FileHash) -> String {
    return format!("{}file/{}", base_url, hash.to_string());
}
