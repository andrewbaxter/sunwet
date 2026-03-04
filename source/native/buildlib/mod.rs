pub mod dbv0;
pub mod dbv1;

use {
    good_ormning::sqlite::types::Type,
};

#[derive(Clone)]
pub struct BuildDbInput {
    pub node_type: Type,
    pub node_array_type: Type,
    pub filehash_type: Type,
    pub access_source_type: Type,
}
