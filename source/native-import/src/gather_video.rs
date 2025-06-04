use {
    crate::gather::{
        Gather,
        GatherTrackType,
    },
    loga::ResultContext,
    serde::Deserialize,
    std::path::Path,
};

#[derive(Deserialize)]
struct MkvNode {
    #[serde(default)]
    id: String,
    #[serde(default)]
    children: Vec<MkvNode>,
    #[serde(default)]
    value: Option<String>,
}

impl MkvNode {
    fn find_child_with_id(&self, id: &str) -> Option<&MkvNode> {
        for child in &self.children {
            if child.id == id {
                return Some(child);
            }
        }
        return None;
    }
}

pub fn gather(path: &Path) -> Result<Gather, loga::Error> {
    let mut g = Gather::new(GatherTrackType::Video);
    let elements = match mkvdump::parse_elements_from_file(path, false) {
        Ok(e) => e,
        Err(e) => {
            return Err(loga::err(e.to_string()).context("Unable to read metadata in video file"));
        },
    };

    // Must access untyped json values due to
    // (https://github.com/cadubentzen/mkvdump/issues/138)
    let root =
        serde_json::from_value::<MkvNode>(
            serde_json::to_value(&mkvparser::tree::build_element_trees(&elements)).unwrap(),
        ).context("Mkv metadata not in anticipated structure")?;
    let segment = root.find_child_with_id("Segment").context("Mkvdump root missing Segment node")?;

    // Read tags
    for tag in &segment.find_child_with_id("Tags").context("Mkvdump Segment missing Tags node")?.children {
        let mut levels = vec![];
        let mut tags = vec![];

        // Unify tag data from multiple schemas
        for tag_prop in &tag.children {
            match tag_prop.id.as_str() {
                "Targets" => {
                    for prop_kv in &tag_prop.children {
                        let Some(value) = &prop_kv.value else {
                            continue;
                        };
                        levels.push(value.clone());
                    }
                },
                "SimpleTag" => {
                    fn read_simpletag<'a>(prop: &'a MkvNode) -> Option<(&'a String, &'a String)> {
                        return Option::zip(
                            prop.find_child_with_id("TagName").and_then(|x| x.value.as_ref()),
                            prop.find_child_with_id("TagString").and_then(|x| x.value.as_ref()),
                        );
                    }

                    let Some((k, v)) = read_simpletag(tag_prop) else {
                        continue;
                    };
                    let parent_tag = k.clone();
                    tags.push((k.clone(), v.clone()));

                    // Looks like an encoder bug? Or mkvdump bug?
                    for subprop in &tag_prop.children {
                        if subprop.id != "SimpleTag" {
                            continue;
                        }
                        let Some((k, v)) = read_simpletag(subprop) else {
                            continue;
                        };
                        tags.push((format!("{}__{}", parent_tag, k), v.clone()));
                    }
                },
                _ => {
                    continue;
                },
            }
        }

        // Parse tags
        for level in &levels {
            match level.as_str() {
                "EDITION / ISSUE / VOLUME / OPUS / SEASON / SEQUEL" | "fake_ALBUM" => {
                    for (k, v) in &tags {
                        match k.as_str() {
                            "TITLE" => {
                                g.album_name = Some(v.clone());
                            },
                            "ARTIST" => {
                                g.album_artist.insert(v.clone());
                            },
                            _ => { },
                        }
                    }
                },
                "TRACK / SONG / CHAPTER" => {
                    for (k, v) in &tags {
                        match k.as_str() {
                            "TITLE" => {
                                g.track_name = Some(v.clone());
                            },
                            "ARTIST" => {
                                g.track_artist.push(v.clone());
                            },
                            "PART_NUMBER" => {
                                g.track_index = Some(usize::from_str_radix(&v, 10)?);
                            },
                            _ => { },
                        }
                    }
                },
                _ => { },
            }
        }
    }

    // Read cover art
    //
    // Note: Can't, mkvdump skips all binary fields atm
    // https://github.com/cadubentzen/mkvdump/issues/149
    return Ok(g);
}
