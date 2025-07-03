use {
    super::gather::{
        Gather,
        GatherMedia,
    },
    image::EncodableLayout,
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    std::{
        path::Path,
        process::Command,
        str::FromStr,
    },
};

pub fn gather(path: &Path) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherMedia::Video);
    let mut cmd = Command::new("mkvextract");
    cmd.arg(path).arg("tags").arg("-");
    let elements = cmd.output().context_with("Error extracting mkv tags", ea!(command = cmd.dbg_str()))?;
    if !elements.status.success() {
        return Err(loga::err_with("Error extracting mkv tags", ea!(command = cmd.dbg_str())));
    }
    let elements =
        xmltree::Element::parse(&mut elements.stdout.as_bytes()).context("Malformed mkvextract output xml")?;
    for tag in elements.children {
        let Some(tag) = tag.as_element() else {
            continue;
        };
        let Some(targets) = tag.get_child("Targets") else {
            continue;
        };
        let Some(target_type) = targets.get_child("TargetType") else {
            continue;
        };
        let Some(level) = target_type.get_text() else {
            continue;
        };
        for child in &tag.children {
            let Some(child) = child.as_element() else {
                continue;
            };
            if child.name != "Simple" {
                continue;
            }
            let Some(k) = child.get_child("Name").and_then(|x| x.get_text()) else {
                continue;
            };
            let Some(v) = child.get_child("String").and_then(|x| x.get_text()) else {
                continue;
            };
            match level.as_ref() {
                "COLLECTION" => {
                    match k.as_ref() {
                        "TITLE" => {
                            g.album_name = Some(v.to_string());
                        },
                        "ARTIST" => {
                            g.album_artist.insert(v.to_string());
                        },
                        _ => { },
                    }
                },
                "ALBUM" | "OPERA" | "CONCERT" | "MOVIE" | "EPISODE" => {
                    match k.as_ref() {
                        "TITLE" => {
                            g.track_name = Some(v.to_string());
                        },
                        "ARTIST" => {
                            g.track_artist.push(v.to_string());
                        },
                        "PART_NUMBER" => {
                            g.track_index = Some(f64::from_str(&v)?);
                        },
                        _ => { },
                    }
                },
                _ => { },
            }
        }
    }
    return Ok(g);
}
