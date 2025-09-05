use {
    loga::ResultContext,
    sha2::{
        Digest,
        Sha256,
    },
    std::{
        collections::{
            BTreeSet,
            HashMap,
        },
        path::{
            Path,
            PathBuf,
        },
    },
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GatherMedia {
    Audio,
    Video,
    Comic,
    Book,
}

pub struct Gather {
    pub album_name: Option<String>,
    pub album_artist: BTreeSet<String>,
    pub track_artist: Vec<String>,
    pub track_name: Option<String>,
    pub track_index: Option<f64>,
    pub track_type: GatherMedia,
    pub track_superindex: Option<f64>,
    pub track_language: Option<String>,
    pub track_cover: HashMap<usize, PathBuf>,
}

impl Gather {
    pub fn new(type_: GatherMedia) -> Gather {
        return Gather {
            album_name: Default::default(),
            album_artist: Default::default(),
            track_artist: Default::default(),
            track_name: Default::default(),
            track_index: Default::default(),
            track_type: type_,
            track_superindex: Default::default(),
            track_language: Default::default(),
            track_cover: Default::default(),
        }
    }
}

pub fn prep_cover(sunwet_dir: &Path, mime: &str, data: &[u8]) -> Result<Option<PathBuf>, loga::Error> {
    let suffix = match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/gif" => "gif",
        "image/tiff" => "tif",
        _ => {
            return Ok(None);
        },
    };
    let digest = hex::encode(Sha256::digest(data));
    let path = sunwet_dir.join(format!("{}.{}", digest, suffix));
    if !path.exists() {
        std::fs::write(&path, data).context("Error writing cover from file")?;
    }
    return Ok(Some(path));
}
