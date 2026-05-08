pub mod dbv0;
pub mod dbv1;
pub mod dbv2;
pub mod dbv3;

#[derive(Clone)]
pub struct BuildDbInput {
    pub node_type_path: &'static str,
    pub filehash_type_path: &'static str,
    pub access_source_type_path: &'static str,
}
