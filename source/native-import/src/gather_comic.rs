use {
    crate::gather::{
        prep_cover,
        Gather,
        GatherTrackType,
    },
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    mime_guess::Mime,
    regex::Regex,
    std::{
        path::Path,
        process::Command,
    },
};

fn extract(archive: &Path, subpath: &str) -> Result<Vec<u8>, loga::Error> {
    let mut cmd = Command::new("7zz");
    cmd.arg("e");
    cmd.arg(archive);
    cmd.arg("-so");
    cmd.arg(subpath);
    let output =
        cmd
            .output()
            .context_with(
                "Error reading file in comic book archive",
                ea!(path = archive.dbg_str(), subpath = subpath, cmd = cmd.dbg_str()),
            )?;
    if !output.status.success() {
        return Err(
            loga::err_with(
                "Error reading file in comic book archive",
                ea!(path = archive.dbg_str(), subpath = subpath, cmd = cmd.dbg_str(), output = output.dbg_str()),
            ),
        );
    }
    return Ok(output.stdout);
}

fn text(e: &xmltree::Element, k: &str) -> Option<String> {
    let Some(v) = e.get_child(k) else {
        return None;
    };
    let Some(t) = v.get_text() else {
        return None;
    };
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    return Some(t.to_string());
}

pub fn gather(sunwet_dir: &Path, path: &Path) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherTrackType::Comic);
    let mut list_cmd = Command::new("7zz");
    list_cmd.arg("l");
    list_cmd.arg("-ba");
    list_cmd.arg(path);
    let list_output =
        list_cmd
            .output()
            .context_with("Error listing contents of comic book archive", ea!(cmd = list_cmd.dbg_str()))?;
    if !list_output.status.success() {
        return Err(
            loga::err_with(
                "Error listing comic archive contents",
                ea!(cmd = list_cmd.dbg_str(), output = list_output.dbg_str()),
            ),
        );
    }
    let index_matcher = Regex::new("(\\d+)").unwrap();
    let mut cover: Option<(Vec<usize>, &str, Mime)> = None;
    let lines = String::from_utf8_lossy(&list_output.stdout);
    for line in lines.lines() {
        let Some(line) = line.get(53..) else {
            // Filter other column junk... pray that this never changes (please output json
            // 7zz)
            continue;
        };
        let mime = mime_guess::from_path(line).first_or_octet_stream();
        if line.to_ascii_lowercase().ends_with("comicinfo.xml") {
            let comicinfo =
                xmltree::Element::parse(
                    &mut extract(path, line).context("Error extracting comicinfo.xml")?.as_slice(),
                ).context("Malformed comicinfo.xml")?;
            g.track_name = text(&comicinfo, "Title");
            for k in ["Writer", "Penciler", "Inker", "Colorist", "Letterer", "CoverArtist"] {
                if let Some(artist) = text(&comicinfo, k) {
                    g.track_artist.push(artist);
                }
            }
            if let Some(album) = text(&comicinfo, "Series") {
                g.album_name = Some(album);
            }
            if let Some(number) = text(&comicinfo, "Number") {
                let Ok(index) = usize::from_str_radix(&number, 10) else {
                    return Err(loga::err(format!("Epub has invalid index (display-seq): [{}]", number)));
                };
                g.track_index = Some(index);
            }
            if let Some(lang) = text(&comicinfo, "LanguageISO") {
                g.track_language = Some(lang);
            }
        } else if mime.type_().as_str() == "image" {
            let mut sort = vec![];
            for seg in index_matcher.captures_iter(line) {
                sort.push(usize::from_str_radix(seg.get(1).unwrap().as_str(), 10).unwrap_or(usize::MAX));
            }
            cover = Some(cover.filter(|(p, ..)| *p <= sort).unwrap_or_else(|| (sort, line, mime)));
        }
    }
    if let Some((_, cover, mime)) = cover {
        if let Some(path) =
            prep_cover(
                &sunwet_dir,
                &mime.essence_str(),
                &extract(path, cover).context("Error extracting cover from comic archive")?,
            )? {
            g
                .track_cover
                .insert(g.track_superindex.unwrap_or_default() * 10000 + g.track_index.unwrap_or(1000), path);
        }
    }
    return Ok(g);
}
