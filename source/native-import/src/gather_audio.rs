use {
    crate::gather::{
        prep_cover,
        Gather,
        GatherTrackType,
    },
    flowcontrol::shed,
    loga::ErrContext,
    std::{
        ffi::OsStr,
        fs::File,
        path::Path,
    },
};

pub fn gather(sunwet_dir: &Path, path: &Path, e: &OsStr) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherTrackType::Audio);
    let mut info =
        match symphonia
        ::default
        ::get_probe().format(
            &symphonia::core::probe::Hint::new().with_extension(&e.to_str().unwrap()),
            symphonia::core::io::MediaSourceStream::new(Box::new(File::open(path)?), Default::default()),
            &Default::default(),
            &Default::default(),
        ) {
            Ok(i) => i,
            Err(e) => {
                return Err(e.context("Unable to read audio file"));
            },
        };
    let mut parse_metadata = |metadata: &symphonia::core::meta::MetadataRevision| {
        for tag in metadata.tags() {
            match tag.std_key {
                Some(k) => match k {
                    symphonia::core::meta::StandardTagKey::Album => {
                        g.album_name = Some(tag.value.to_string());
                    },
                    symphonia::core::meta::StandardTagKey::AlbumArtist => {
                        g.album_artist.insert(tag.value.to_string());
                    },
                    symphonia::core::meta::StandardTagKey::Artist => {
                        g.track_artist.push(tag.value.to_string());
                    },
                    symphonia::core::meta::StandardTagKey::DiscNumber => {
                        g.track_superindex = Some(usize::from_str_radix(&tag.value.to_string(), 10)?);
                    },
                    symphonia::core::meta::StandardTagKey::TrackNumber => {
                        g.track_index =
                            Some(usize::from_str_radix(&tag.value.to_string().split("/").next().unwrap(), 10)?);
                    },
                    symphonia::core::meta::StandardTagKey::TrackTitle => {
                        g.track_name = Some(tag.value.to_string());
                    },
                    _ => { },
                },
                None => { },
            }
        }
        for v in metadata.visuals() {
            let priority = match v.usage {
                Some(u) => match u {
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
                },
                None => 1000,
            };
            if let Some(path) = prep_cover(&sunwet_dir, &v.media_type, &v.data)? {
                g.track_cover.insert(priority, path);
            }
        }
        return Ok(()) as Result<(), loga::Error>;
    };
    shed!{
        let Some(metadata) = info.metadata.get() else {
            break;
        };
        let Some(metadata) = metadata.current() else {
            break;
        };
        parse_metadata(metadata)?;
    }
    if let Some(metadata) = info.format.metadata().current() {
        parse_metadata(metadata)?;
    }
    return Ok(g);
}
