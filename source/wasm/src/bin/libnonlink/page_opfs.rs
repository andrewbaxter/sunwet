use {
    crate::libnonlink::state::{
        set_page,
        state,
    },
    js_sys::Reflect,
    lunk::ProcessingContext,
    rooting::El,
    shared_wasm::{
        log::Log,
        opfs::{
            OpfsAmbig,
            OpfsDir,
            opfs_root,
        },
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm::js::{
        el_async,
        on_thinking,
        style_export,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemRemoveOptions,
    },
};

fn format_size(bytes: f64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} B", bytes as u64)
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn details_is_open(el: &El) -> bool {
    Reflect::get(&el.raw(), &JsValue::from_str("open")).ok().and_then(|v| v.as_bool()).unwrap_or(false)
}

struct MarkedFiles {
    entries: Vec<(OpfsDir, String)>,
}

fn build_delete_toggle(marked: &Rc<RefCell<MarkedFiles>>, parent_dir: &OpfsDir, name: &str) -> El {
    let btn = style_export::leaf_opfs_delete_toggle().root;
    let class_pressed = style_export::class_state_pressed().value;
    btn.ref_on("click", {
        let marked = marked.clone();
        let parent_dir = parent_dir.clone();
        let name = name.to_string();
        let btn = btn.clone();
        move |_| {
            let mut m = marked.borrow_mut();
            let existing = m.entries.iter().position(|(d, n)| {
                JsValue::from(&d.1) == JsValue::from(&parent_dir.1) && n == &name
            });
            if let Some(idx) = existing {
                m.entries.remove(idx);
                btn.ref_modify_classes(&[(&class_pressed, false)]);
            } else {
                m.entries.push((parent_dir.clone(), name.clone()));
                btn.ref_modify_classes(&[(&class_pressed, true)]);
            }
        }
    });
    btn
}

fn build_file_entry(
    log: &Rc<dyn Log>,
    marked: &Rc<RefCell<MarkedFiles>>,
    parent_dir: &OpfsDir,
    name: &str,
    ambig: &OpfsAmbig,
) -> El {
    let file = ambig.file();
    match file {
        Ok(file) => {
            let log = log.clone();
            let marked = marked.clone();
            let parent_dir = parent_dir.clone();
            let name = name.to_string();
            el_async(async move {
                let size_str = match file.size().await {
                    Ok(size) => format_size(size),
                    Err(_) => "?".to_string(),
                };
                let comp =
                    style_export::cont_opfs_file(
                        style_export::ContOpfsFileArgs { name: format!("{} ({})", name, size_str) },
                    );
                let loaded = Rc::new(RefCell::new(false));
                comp.root.ref_on("toggle", {
                    let root = comp.root.clone();
                    let body = comp.body.clone();
                    let file = file.clone();
                    let log = log.clone();
                    let marked = marked.clone();
                    let parent_dir = parent_dir.clone();
                    let name = name.clone();
                    move |_| {
                        if !details_is_open(&root) || *loaded.borrow() {
                            return;
                        }
                        *loaded.borrow_mut() = true;
                        let file = file.clone();
                        let log = log.clone();
                        let marked = marked.clone();
                        let parent_dir = parent_dir.clone();
                        let name = name.clone();
                        body.ref_push(el_async(async move {
                            let mut els = vec![build_delete_toggle(&marked, &parent_dir, &name)];
                            match file.read_binary_prefix(1000).await {
                                Ok(data) => {
                                    if !data.is_empty() {
                                        let text = String::from_utf8_lossy(&data);
                                        els.push(
                                            style_export::leaf_opfs_file_preview(
                                                style_export::LeafOpfsFilePreviewArgs { text: text.into_owned() },
                                            ).root,
                                        );
                                    }
                                },
                                Err(e) => {
                                    log.log(&format!("Error reading file {}: {}", name, e));
                                },
                            }
                            Ok::<_, String>(els)
                        }));
                    }
                });
                Ok::<_, String>(vec![comp.root])
            })
        },
        Err(_) => {
            let comp =
                style_export::cont_opfs_file(
                    style_export::ContOpfsFileArgs { name: format!("{} (unknown type)", name) },
                );
            comp.root
        },
    }
}

fn build_dir_entry(
    log: &Rc<dyn Log>,
    marked: &Rc<RefCell<MarkedFiles>>,
    parent_dir: &OpfsDir,
    name: &str,
    ambig: &OpfsAmbig,
    depth: u32,
) -> El {
    let comp = style_export::cont_opfs_dir(style_export::ContOpfsDirArgs {
        depth: depth as usize,
        name: name.to_string(),
    });
    let dir = match ambig.dir() {
        Ok(d) => d,
        Err(_) => return comp.root,
    };
    let loaded = Rc::new(RefCell::new(false));
    comp.root.ref_on("toggle", {
        let root = comp.root.clone();
        let body = comp.body.clone();
        let log = log.clone();
        let marked = marked.clone();
        let parent_dir = parent_dir.clone();
        let name = name.to_string();
        move |_| {
            if !details_is_open(&root) || *loaded.borrow() {
                return;
            }
            *loaded.borrow_mut() = true;
            let log = log.clone();
            let marked = marked.clone();
            let parent_dir = parent_dir.clone();
            let name = name.clone();
            let dir = dir.clone();
            body.ref_push(el_async(async move {
                let mut els = vec![build_delete_toggle(&marked, &parent_dir, &name)];
                match dir.list(&log).await {
                    Ok(entries) => {
                        let mut children = vec![];
                        for (child_name, child_ambig) in &entries {
                            if child_ambig.1.dyn_ref::<FileSystemDirectoryHandle>().is_some() {
                                children.push(
                                    build_dir_entry(&log, &marked, &dir, child_name, child_ambig, depth + 1),
                                );
                            } else {
                                children.push(build_file_entry(&log, &marked, &dir, child_name, child_ambig));
                            }
                        }
                        if entries.is_empty() {
                            children.push(style_export::leaf_opfs_empty().root);
                        }
                        els.push(
                            style_export::cont_opfs_children(
                                style_export::ContOpfsChildrenArgs { children: children },
                            ).root,
                        );
                    },
                    Err(e) => {
                        log.log(&format!("Error listing dir {}: {}", name, e));
                    },
                }
                Ok::<_, String>(els)
            }));
        }
    });
    comp.root
}

fn build_root_entries(log: Rc<dyn Log>, marked: Rc<RefCell<MarkedFiles>>) -> El {
    el_async(async move {
        let root = opfs_root().await;
        let mut els = vec![];
        match root.list(&log).await {
            Ok(entries) => {
                for (name, ambig) in &entries {
                    if ambig.1.dyn_ref::<FileSystemDirectoryHandle>().is_some() {
                        els.push(build_dir_entry(&log, &marked, &root, name, ambig, 0));
                    } else {
                        els.push(build_file_entry(&log, &marked, &root, name, ambig));
                    }
                }
                if entries.is_empty() {
                    els.push(style_export::leaf_opfs_empty().root);
                }
            },
            Err(e) => {
                log.log(&format!("Error listing OPFS root: {}", e));
            },
        }
        Ok::<_, String>(els)
    })
}

pub fn build_page_opfs(pc: &mut ProcessingContext) {
    let log = state().log.clone();
    let marked: Rc<RefCell<MarkedFiles>> = Rc::new(RefCell::new(MarkedFiles { entries: vec![] }));
    let entries_container =
        style_export::cont_group(
            style_export::ContGroupArgs { children: vec![build_root_entries(log.clone(), marked.clone())] },
        ).root;
    let delete_button = style_export::leaf_button_big_commit().root;
    on_thinking(&delete_button, {
        let marked = marked.clone();
        let entries_container = entries_container.clone();
        let log = log.clone();
        move || {
            let marked = marked.clone();
            let entries_container = entries_container.clone();
            let log = log.clone();
            async move {
                let entries: Vec<(OpfsDir, String)> = {
                    let mut m = marked.borrow_mut();
                    m.entries.drain(..).collect()
                };
                if entries.is_empty() {
                    return;
                }
                for (parent, name) in entries {
                    let _ = JsFuture::from(parent.1.remove_entry_with_options(&name, &{
                        let mut o = FileSystemRemoveOptions::new();
                        o.recursive(true);
                        o
                    })).await;
                }
                entries_container.ref_clear();
                entries_container.ref_push(build_root_entries(log, marked));
            }
        }
    });
    let page = style_export::cont_page_opfs(style_export::ContPageOpfsArgs {
        bar_children: vec![delete_button],
        children: vec![entries_container],
    });
    set_page(pc, "OPFS", page.root);
}
