use {
    crate::libnonlink::state::state,
    gloo::utils::window,
    js_sys::{
        Array,
        Uint8Array,
    },
    serde::{
        Serialize,
        de::DeserializeOwned,
    },
    tokio_stream::StreamExt,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        JsFuture,
        stream::JsStream,
    },
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
        FileSystemWritableFileStream,
    },
};

// # Opfs utils
pub async fn opfs_root() -> FileSystemDirectoryHandle {
    return JsFuture::from(window().navigator().storage().get_directory())
        .await
        .expect("Error getting opfs root")
        .dyn_into::<FileSystemDirectoryHandle>()
        .unwrap();
}

pub async fn opfs_ensure_dir(parent: &FileSystemDirectoryHandle, seg: &str) -> FileSystemDirectoryHandle {
    return JsFuture::from(parent.get_directory_handle_with_options(seg, &{
        let x = FileSystemGetDirectoryOptions::new();
        x.set_create(true);
        x
    }))
        .await
        .expect("Error getting/creating opfs dir")
        .dyn_into::<FileSystemDirectoryHandle>()
        .expect("Opfs get dir handle result wasn't file system dir handle");
}

/// Each JsValue is either FileSystemDirectoryHandle or FileSystemFileHandle
pub async fn opfs_list_dir(parent: &FileSystemDirectoryHandle) -> Vec<(String, JsValue)> {
    let mut entries = vec![];
    let mut entries0 = JsStream::from(parent.entries());
    while let Some(e) = entries0.next().await {
        let e = match e {
            Ok(e) => e,
            Err(e) => {
                state().log.log_js2("Error reading directory entry", parent, &e);
                continue;
            },
        };
        let e = e.dyn_into::<Array>().unwrap();
        let name = e.get(0).as_string().unwrap();
        let handle = e.get(1);
        entries.push((name, handle));
    }
    return entries;
}

pub async fn opfs_read_binary(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<Vec<u8>, String> {
    return Ok(
        JsFuture::from(
            web_sys::File::from(
                JsFuture::from(
                    FileSystemFileHandle::from(
                        JsFuture::from(parent.get_file_handle(seg))
                            .await
                            .map_err(|e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()))?,
                    ).get_file(),
                )
                    .await
                    .map_err(
                        |e| format!("Error getting file from file handle at seg [{}]: {:?}", seg, e.as_string()),
                    )?,
            ).text(),
        )
            .await
            .map_err(|e| format!("Error getting string contents of file at seg [{}]: {:?}", seg, e.as_string()))?
            .dyn_into::<Uint8Array>()
            .unwrap()
            .to_vec(),
    );
}

pub async fn opfs_read_json<
    T: DeserializeOwned,
>(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<T, String> {
    return Ok(
        serde_json::from_slice::<T>(
            &opfs_read_binary(parent, seg).await?,
        ).map_err(|e| format!("Error parsing json file from opfs at seg [{}]: {}", seg, e))?,
    );
}

pub async fn opfs_write_binary(parent: &FileSystemDirectoryHandle, seg: &str, data: &Vec<u8>) -> Result<(), String> {
    let f =
        FileSystemFileHandle::from(
            JsFuture::from(parent.get_file_handle(seg))
                .await
                .map_err(|e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    let w =
        FileSystemWritableFileStream::from(
            JsFuture::from(f.create_writable())
                .await
                .map_err(|e| format!("Error getting file handle writable at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    JsFuture::from(
        w
            .write_with_u8_array(data)
            .map_err(|e| format!("Error writing message to opfs file at seg [{}]: {:?}", seg, e.as_string()))?,
    )
        .await
        .map_err(|e| format!("Error writing message to opfs file at seg [{}] (2): {:?}", seg, e.as_string()))?;
    return Ok(());
}

pub async fn opfs_write_json<
    T: Serialize,
>(parent: &FileSystemDirectoryHandle, seg: &str, data: T) -> Result<(), String> {
    return opfs_write_binary(parent, seg, &serde_json::to_vec(&data).unwrap()).await;
}

pub async fn opfs_delete(parent: &FileSystemDirectoryHandle, seg: &str) {
    if let Err(e) = JsFuture::from(parent.remove_entry(seg)).await {
        state().log.log_js(&format!("Error deleting opfs entry at [{}]", seg), &e);
    }
}

pub async fn opfs_exists(parent: &FileSystemDirectoryHandle, seg: &str) -> bool {
    // Bikeshedding https://github.com/whatwg/fs/issues/80
    //
    // This is mostly used by offline, offline is mostly for mobile devices with
    // limited storage and/or in small directories - so hopefully this hack doesn't
    // blow up for typical use cases.
    for (k, _) in opfs_list_dir(parent).await {
        if k == seg {
            return true;
        }
    }
    return false;
}
