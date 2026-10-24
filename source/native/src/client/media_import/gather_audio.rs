use {
    loga::ErrContext,
    std::{
        ffi::OsStr,
        fs::File,
        path::Path,
    },
    super::gather::{
        Gather,
        GatherMedia,
        prep_cover,
    },
};

pub fn gather(sunwet_dir: &Path, path: &Path, e: &OsStr) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherMedia::Audio);
    let mut format =
        match symphonia
        ::default
        ::get_probe().probe(
            &symphonia::core::formats::probe::Hint::new().with_extension(&e.to_str().unwrap()),
            symphonia::core::io::MediaSourceStream::new(Box::new(File::open(path)?), Default::default()),
            Default::default(),
            Default::default(),
        ) {
            Ok(i) => i,
            Err(e) => {
                return Err(e.context("Unable to read audio file"));
            },
        };
    let mut parse_metadata = |metadata: &symphonia::core::meta::MetadataContainer| {
        for tag in &metadata.tags {
            match &tag.std {
                None => {
                    // Some encoders write track-scoped tags (like the track number) into a
                    // single flat tag block with no explicit target, which symphonia defaults
                    // to the "ALBUM" target rather than "TRACK"/"SONG", so it fails to map them
                    // to a standard tag. Also, some tag names (a bare "ALBUM" or "ALBUM_ARTIST"/
                    // "ALBUMARTIST" or "DISCNUMBER"/"DISC") are never mapped by symphonia's
                    // Matroska tag mapper regardless of target, since the Matroska tag spec
                    // expects those as a "TITLE"/"ARTIST"/"PART_NUMBER" tag nested under an
                    // explicit album/release-level `Targets` context rather than as their own
                    // flat tag name - but most taggers (ffmpeg, mkvpropedit, etc.) write them
                    // flat. Fall back to matching the raw tag name directly in these cases.
                    if let symphonia::core::meta::RawValue::String(v) = &tag.raw.value {
                        let name = tag.raw.key.rsplit('@').next().unwrap_or(&tag.raw.key);
                        if g.track_index.is_none() && name.eq_ignore_ascii_case("PART_NUMBER") {
                            if let Ok(n) = v.parse::<f64>() {
                                g.track_index = Some(n);
                            }
                        }
                        if g.album_name.is_none() && name.eq_ignore_ascii_case("ALBUM") {
                            g.album_name = Some(v.to_string());
                        }
                        if name.eq_ignore_ascii_case("ALBUM_ARTIST") || name.eq_ignore_ascii_case("ALBUMARTIST") {
                            g.album_artist.insert(v.to_string());
                        }
                        if g.track_superindex.is_none() &&
                            (name.eq_ignore_ascii_case("DISCNUMBER") || name.eq_ignore_ascii_case("DISC")) {
                            if let Ok(n) = v.parse::<f64>() {
                                g.track_superindex = Some(n);
                            }
                        }
                    }
                },
                Some(std) => {
                    #[deny(clippy::wildcard_enum_match_arm)]
                    match std {
                        symphonia::core::meta::StandardTag::Album(v) => {
                            g.album_name = Some(v.to_string());
                        },
                        symphonia::core::meta::StandardTag::AlbumArtist(v) => {
                            g.album_artist.insert(v.to_string());
                        },
                        symphonia::core::meta::StandardTag::Artist(v) => {
                            g.track_artist.push(v.to_string());
                        },
                        symphonia::core::meta::StandardTag::DiscNumber(v) => {
                            g.track_superindex = Some(*v as f64);
                        },
                        symphonia::core::meta::StandardTag::TrackNumber(v) => {
                            g.track_index = Some(*v as f64);
                        },
                        symphonia::core::meta::StandardTag::TrackTitle(v) => {
                            g.track_name = Some(v.to_string());
                        },
                        _ => { },
                    }
                },
            }
        }
        for v in &metadata.visuals {
            let priority = match v.usage {
                None => 1000,
                Some(u) => {
                    #[deny(clippy::wildcard_enum_match_arm)]
                    match u {
                        symphonia::core::meta::StandardVisualKey::FrontCover => 0,
                        symphonia::core::meta::StandardVisualKey::Media => 10,
                        symphonia::core::meta::StandardVisualKey::Illustration => 20,
                        symphonia::core::meta::StandardVisualKey::BandArtistLogo => 30,
                        symphonia::core::meta::StandardVisualKey::Leaflet => 40,
                        symphonia::core::meta::StandardVisualKey::FileIcon => 500,
                        symphonia::core::meta::StandardVisualKey::OtherIcon => 500,
                        symphonia::core::meta::StandardVisualKey::BackCover => 500,
                        symphonia::core::meta::StandardVisualKey::LeadArtistPerformerSoloist => 500,
                        symphonia::core::meta::StandardVisualKey::ArtistPerformer => 500,
                        symphonia::core::meta::StandardVisualKey::Conductor => 500,
                        symphonia::core::meta::StandardVisualKey::BandOrchestra => 500,
                        symphonia::core::meta::StandardVisualKey::Composer => 500,
                        symphonia::core::meta::StandardVisualKey::Lyricist => 500,
                        symphonia::core::meta::StandardVisualKey::RecordingLocation => 500,
                        symphonia::core::meta::StandardVisualKey::RecordingSession => 500,
                        symphonia::core::meta::StandardVisualKey::Performance => 500,
                        symphonia::core::meta::StandardVisualKey::ScreenCapture => 500,
                        symphonia::core::meta::StandardVisualKey::PublisherStudioLogo => 500,
                        symphonia::core::meta::StandardVisualKey::Other => 500,
                        _ => 500,
                    }
                },
            };
            if let Some(media_type) = &v.media_type {
                if let Some(path) = prep_cover(&sunwet_dir, media_type, &v.data)? {
                    g.track_cover.insert(priority, path);
                }
            }
        }
        return Ok(()) as Result<(), loga::Error>;
    };
    if let Some(metadata) = format.metadata().current() {
        parse_metadata(&metadata.media)?;
        for per_track in &metadata.per_track {
            parse_metadata(&per_track.metadata)?;
        }
    }
    return Ok(g);
}
