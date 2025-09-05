pub const PREDICATE_IS: &str = "sunwet/1/is";

/// Child of album, indicates an ordered list of primary media.
pub const PREDICATE_TRACK: &str = "sunwet/1/track";

/// Order of track within an album's track list.
pub const PREDICATE_INDEX: &str = "sunwet/1/index";

/// Order of the track within an album's track list -- this is like a disk index,
/// and should be omitted if there's only one disk.
pub const PREDICATE_SUPERINDEX: &str = "sunwet/1/superindex";

/// When the entity was added -- mostly for ID nodes (albums, artists, tracks,
/// notes)
pub const PREDICATE_ADD_TIMESTAMP: &str = "sunwet/1/add_timestamp";
pub const PREDICATE_NAME: &str = "sunwet/1/name";
pub const PREDICATE_ARTIST: &str = "sunwet/1/artist";
pub const PREDICATE_COVER: &str = "sunwet/1/cover";

/// An associated, unordered, non-track file
pub const PREDICATE_DOC: &str = "sunwet/1/doc";
pub const PREDICATE_FILE: &str = "sunwet/1/file";
pub const PREDICATE_MEDIA: &str = "sunwet/1/media";
pub const PREDICATE_TOPIC: &str = "sunwet/1/topic";

/// ISO 639-1 2-letter language
pub const PREDICATE_LANG: &str = "sunwet/1/language";

/// A collection of things.
pub const OBJ_IS_ALBUM: &str = "sunwet/1/album";

/// A thing.
pub const OBJ_IS_TRACK: &str = "sunwet/1/track";

/// Someone who created a thing.
pub const OBJ_IS_ARTIST: &str = "sunwet/1/artist";

/// A document; any non-media file associated with an album.
pub const OBJ_IS_DOC: &str = "sunwet/1/doc";

/// A note... some text you wrote.
pub const OBJ_IS_NOTE: &str = "sunwet/1/note";
pub const OBJ_MEDIA_AUDIO: &str = "sunwet/1/audio";
pub const OBJ_MEDIA_VIDEO: &str = "sunwet/1/video";
pub const OBJ_MEDIA_COMIC: &str = "sunwet/1/comic";
pub const OBJ_MEDIA_BOOK: &str = "sunwet/1/book";
pub const OBJ_MEDIA_IMAGE: &str = "sunwet/1/image";
