//! This is a very basic, very minimal ontology required to bootstrap sunwet and
//! make it useful. It discards lots of useful information.
//!
//! Overview:
//!
//! * The ontology manages "entities" (a node that's a uuid) with associated properties
//!   and relations.
//!
//! * Media entities are grouped together into "albums"
//!
//! * Albums have media entities as tracks
//!
//! * Each media entity has a file
//!
//! * Notes are independent
//!
//! ### -
/// * Subject: any entity
///
/// * Object: one of the enumerated "is" objects values below, or a custom value
///
/// * Plurality: many
///
/// Used to classify objects in a query. I.e. if you have music albums and video
/// albums (tv series), use `is` in a audio album query to only select audio albums.
pub const PREDICATE_IS: &str = "sunwet/1/is";

/// * Subject: any entity
///
/// * Object: any entity
///
/// * Plurality: zero or one
///
/// Used to create a "pointer" entity. The "pointer" entity can have additional or
/// overriding properties.
///
/// An example usage would be a track in a playlist. The track may already have an
/// index within its album, so to order it within the playlist a pointer entity is
/// created with the new index and is added to the playlist instead of the track
/// directly.
pub const PREDICATE_VALUE: &str = "sunwet/1/value";

/// * Subject: an album
///
/// * Object: any entity
///
/// * Plurality: many
///
/// Indicates a list of primary media in the album. E.g. for a music album,
/// `sunwet/1/track` would be used on all of the songs in the album. It would not
/// be used for the cover art, or an included booklet or music video.
///
/// `sunwet/1/index` predicates can be used to order tracks (see below).
pub const PREDICATE_TRACK: &str = "sunwet/1/track";

/// * Subject: a track in an album
///
/// * Object: a number
///
/// * Plurality: one or zero
///
/// The order of a track within an album's track list. Should occur at most once on
/// a track.
pub const PREDICATE_INDEX: &str = "sunwet/1/index";

/// * Subject: a track in an album
///
/// * Object: a number
///
/// * Plurality: one or zero
///
/// The order of the track within an album's track list -- this is like a disk
/// number. This has higher sort priority than `sunwet/1/index`, so a superindex
/// and index of (0, 5) would come before a superindex and index of (1, 2). Sort
/// order is undefined if some tracks have a superindex and some don't.
///
/// Should be omitted if there's only one disk.
pub const PREDICATE_SUPERINDEX: &str = "sunwet/1/superindex";

/// * Subject: any entity
///
/// * Object: a rfc 3339 utc timestamp
///
/// * Plurality: one
///
/// When the entity was added -- mostly for ID nodes (albums, artists, tracks,
/// notes)
pub const PREDICATE_ADD_TIMESTAMP: &str = "sunwet/1/add_timestamp";

/// * Subject: any entity
///
/// * Object: a string
///
/// * Plurality: one or zero
pub const PREDICATE_NAME: &str = "sunwet/1/name";

/// * Subject: any entity
///
/// * Object: any entity
///
/// * Plurality: one or zero
pub const PREDICATE_ARTIST: &str = "sunwet/1/artist";

/// * Subject: any entity
///
/// * Object: an image file hash
///
/// * Plurality: one or zero
pub const PREDICATE_COVER: &str = "sunwet/1/cover";

/// * Subject: any entity
///
/// * Object: a file hash
///
/// * Plurality: many
///
/// Like track, but for any non-primary media, like booklets or artwork.
pub const PREDICATE_DOC: &str = "sunwet/1/doc";

/// * Subject: a media entity
///
/// * Object: a file hash
///
/// * Plurality: one
///
/// This associates a file with a media entity (i.e. the music file with the song
/// entity).
pub const PREDICATE_FILE: &str = "sunwet/1/file";

/// * Subject: a media album or a media entity
///
/// * Object: one of the enumerated "media" object values below, or a custom value.
///
/// * Plurality: many
///
/// This associates a file with a media entity (i.e. the music file with the song
/// entity). When used for an album, this is the primary media type contained
/// within. I.e. for a music album it'd be `sunwet/1/audio` even if there are video
/// files present.
pub const PREDICATE_MEDIA: &str = "sunwet/1/media";

/// * Subject: a note entity
///
/// * Object: any string (free text), used for full text search
///
/// * Plurality: zero or one
///
/// This is used for searching notes, avoiding matches for non-topic words that may
/// appear in the note body. A descriptive title would be a good topic.
pub const PREDICATE_TOPIC: &str = "sunwet/1/topic";

/// * Subject: a note entity
///
/// * Object: ISO 639-1 2-letter language.
///
/// * Plurality: zero or one
///
/// This is the present language of the media, i.e. for searching for books
/// readable in English, or videos with French main audio tracks.
pub const PREDICATE_LANG: &str = "sunwet/1/language";

/// * Subject: any entity
///
/// * Object: JSON `null`
///
/// * Plurality: zero or one
///
/// This is used to soft delete things from the database. The default queries
/// exclude entities with this predicate.
pub const PREDICATE_DELETE: &str = "sunwet/1/delete";

/// Indicates an entity that is an official collection of things.
pub const OBJ_IS_ALBUM: &str = "sunwet/1/album";

/// Indicates an entity that is a user-created collection of things.
pub const OBJ_IS_PLAYLIST: &str = "sunwet/1/playlist";

/// Indicates an entity (person, group, organization) that created a thing.
pub const OBJ_IS_ARTIST: &str = "sunwet/1/artist";

/// A document; any non-media file associated with an album.
pub const OBJ_IS_DOC: &str = "sunwet/1/doc";

/// A note... any freeform text.
pub const OBJ_IS_NOTE: &str = "sunwet/1/note";

/// A media entity with an associated audio file.
pub const OBJ_MEDIA_AUDIO: &str = "sunwet/1/audio";

/// A media entity with an associated video file.
pub const OBJ_MEDIA_VIDEO: &str = "sunwet/1/video";

/// A media entity with an associated cb7/cbz file.
pub const OBJ_MEDIA_COMIC: &str = "sunwet/1/comic";

/// A media entity with an associated epub file.
pub const OBJ_MEDIA_BOOK: &str = "sunwet/1/book";

/// A media entity with an associated image file.
pub const OBJ_MEDIA_IMAGE: &str = "sunwet/1/image";
