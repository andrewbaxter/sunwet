use {
    super::gather::{
        prep_cover,
        Gather,
        GatherTrackType,
    },
    epub::doc::EpubDoc,
    loga::{
        ResultContext,
    },
    std::{
        path::Path,
        str::FromStr,
    },
};

pub fn gather(sunwet_dir: &Path, path: &Path) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherTrackType::Book);
    let mut epub = EpubDoc::new(path).context("Failed to read epub")?;
    if let Some(title) = epub.mdata("title") {
        g.track_name = Some(title.value.clone());
        if let Some(index) = title.refinement("display-seq") {
            let Ok(index) = f64::from_str(&index.value) else {
                return Err(loga::err(format!("Epub has invalid index (display-seq): [{}]", index.value)));
            };
            g.track_index = Some(index);
        }
    }
    if let Some(artist) = epub.mdata("creator") {
        g.track_artist.push(artist.value.clone());
    }
    if let Some(album) = epub.mdata("belongs-to-collection") {
        g.album_name = Some(album.value.clone());
        if let Some(idx) = album.refinement("group-position") {
            let indexes = idx.value.split(".").collect::<Vec<_>>();
            let mut indexes2 = vec![];
            for i in indexes {
                let Ok(i) = f64::from_str(i) else {
                    return Err(loga::err(format!("Epub has invalid index (group-position): [{}]", i)));
                };
                indexes2.push(i);
            }
            indexes2.reverse();
            let mut indexes2 = indexes2.into_iter();
            g.track_index = indexes2.next();
            if let Some(i) = indexes2.next() {
                g.track_superindex = Some(i);
            }
            if indexes2.next().is_some() {
                return Err(loga::err("Currently only up to 2 indexes are supported (group-position)"));
            }
        }
    }
    if let Some((cover_data, mime)) = epub.get_cover() {
        if let Some(path) = prep_cover(&sunwet_dir, &mime, &cover_data)? {
            g
                .track_cover
                .insert(
                    (g.track_superindex.unwrap_or_default() * 10000. + g.track_index.unwrap_or(1000.)) as usize,
                    path,
                );
        }
    }
    return Ok(g);
}
