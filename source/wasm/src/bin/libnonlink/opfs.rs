use {
    crate::libnonlink::state::state,
    gloo::utils::window,
    js_sys::{
        Array,
        JSON,
        Uint8Array,
    },
    serde::{
        Serialize,
        de::DeserializeOwned,
    },
    std::rc::Rc,
    tokio_stream::StreamExt,
    wasm::js::jsstr,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        JsFuture,
        stream::JsStream,
    },
    web_sys::{
        Blob,
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
        FileSystemGetFileOptions,
        FileSystemRemoveOptions,
        FileSystemWritableFileStream,
        Url,
    },
};

pub struct DebugPath_ {
    pub parent: DebugPath,
    pub segs: Vec<String>,
}

#[derive(Clone)]
pub struct DebugPath(pub Option<Rc<DebugPath_>>);

impl DebugPath {
    fn join(&self, segs: Vec<String>) -> Self {
        return Self(Some(Rc::new(DebugPath_ {
            parent: self.clone(),
            segs: segs,
        })));
    }
}

impl std::fmt::Display for DebugPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(s) = &self.0 {
            return format_args!("{}/{:?}", s.parent, s.segs).fmt(f);
        } else {
            return "/".fmt(f);
        }
    }
}

#[derive(Clone)]
pub struct OpfsDir(pub DebugPath, pub FileSystemDirectoryHandle);

impl OpfsDir {
    pub async fn get_dir(&self, segs: Vec<String>) -> Result<OpfsDir, String> {
        let mut at = self.1.clone();
        for seg in &segs {
            at =
                JsFuture::from(at.get_directory_handle(seg))
                    .await
                    .map_err(
                        |e| format!("Error getting directory in opfs (seg [{}] of [{:?}]): {}", seg, segs, jsstr(e)),
                    )?
                    .dyn_into::<FileSystemDirectoryHandle>()
                    .expect("Opfs file get, cast to directory handle");
        }
        return Ok(OpfsDir(self.0.join(segs), at));
    }

    pub async fn ensure_dir(&self, segs: Vec<String>) -> Result<OpfsDir, String> {
        let mut at = self.clone();
        for seg in segs {
            let path = at.0.join(vec![seg.clone()]);
            let out =
                JsFuture::from(at.1.get_directory_handle_with_options(&seg, &{
                    let x = FileSystemGetDirectoryOptions::new();
                    x.set_create(true);
                    x
                }))
                    .await
                    .map_err(|e| format!("Error getting/creating opfs dir [{}]: {}", path, jsstr(e)))?
                    .dyn_into::<FileSystemDirectoryHandle>()
                    .expect("Opfs get dir handle result wasn't file system dir handle");
            at = OpfsDir(path, out);
        }
        return Ok(at);
    }

    pub async fn get_file(&self, mut segs: Vec<String>) -> Result<OpfsFile, String> {
        let Some(file) = segs.pop() else {
            return Err(format!("opfs_get_file called with empty segments"));
        };
        let parent = self.get_dir(segs).await?;
        let path = self.0.join(vec![file.clone()]);
        let out =
            JsFuture::from(parent.1.get_file_handle(&file))
                .await
                .map_err(|e| format!("Error getting file in opfs [{}]: {}", path, jsstr(e)))?
                .dyn_into::<FileSystemFileHandle>()
                .expect("opfs file get, cast to file handle");
        return Ok(OpfsFile(path, out));
    }

    pub async fn ensure_file(&self, mut segs: Vec<String>) -> Result<OpfsWriteFile, String> {
        let Some(file) = segs.pop() else {
            return Err(format!("opfs_get_file called with empty segments"));
        };
        let parent = self.get_dir(segs).await?;
        let path = parent.0.join(vec![file.clone()]);
        let out =
            JsFuture::from(parent.1.get_file_handle_with_options(&file, &{
                let o = FileSystemGetFileOptions::new();
                o.set_create(true);
                o
            }))
                .await
                .map_err(|e| format!("Error getting file in opfs [{}]: {:?}", path, jsstr(e)))?
                .dyn_into::<FileSystemFileHandle>()
                .expect("opfs file get, cast to file handle");
        return Ok(OpfsWriteFile(path, out));
    }

    pub async fn list(&self) -> Result<Vec<(String, OpfsAmbig)>, String> {
        let mut entries = vec![];
        let mut entries0 = JsStream::from(self.1.entries());
        while let Some(e) = entries0.next().await {
            let e = match e {
                Ok(e) => e,
                Err(e) => {
                    state().log.log_js(&format!("Error reading directory entry [{}]", self.0), &e);
                    continue;
                },
            };
            let e = e.dyn_into::<Array>().unwrap();
            let name = e.get(0).as_string().unwrap();
            let handle = e.get(1);
            entries.push((name.clone(), OpfsAmbig(self.0.join(vec![name]), handle)));
        }
        return Ok(entries);
    }

    pub async fn delete(&self, seg: &str) {
        if let Err(e) = JsFuture::from(self.1.remove_entry_with_options(seg, &{
            let o = FileSystemRemoveOptions::new();
            o.set_recursive(true);
            o
        })).await {
            state().log.log_js(&format!("Error deleting opfs entry at [{}]", self.0), &e);
        }
    }

    pub async fn exists(&self, seg: &str) -> Result<bool, String> {
        // Bikeshedding https://github.com/whatwg/fs/issues/80
        //
        // This is mostly used by offline, offline is mostly for mobile devices with
        // limited storage and/or in small directories - so hopefully this hack doesn't
        // blow up for typical use cases.
        for (k, _) in self.list().await? {
            if k == seg {
                return Ok(true);
            }
        }
        return Ok(false);
    }
}

#[derive(Clone)]
pub struct OpfsFile(pub DebugPath, pub FileSystemFileHandle);

impl OpfsFile {
    pub async fn read_binary(&self) -> Result<Vec<u8>, String> {
        return Ok(
            JsFuture::from(
                web_sys::File::from(
                    JsFuture::from(self.1.get_file())
                        .await
                        .map_err(
                            |e| format!("Error getting file from file handle at seg [{}]: {}", self.0, jsstr(e)),
                        )?,
                ).bytes(),
            )
                .await
                .map_err(|e| format!("Error getting string contents of file at seg [{}]: {}", self.0, jsstr(e)))?
                .dyn_into::<Uint8Array>()
                .unwrap()
                .to_vec(),
        );
    }

    pub async fn read_json<T: DeserializeOwned>(&self) -> Result<T, String> {
        return Ok(
            serde_json::from_slice::<T>(
                &self.read_binary().await?,
            ).map_err(|e| format!("Error parsing json file at [{}]: {}", self.0, e))?,
        );
    }

    pub async fn url(&self) -> Result<String, String> {
        return Ok(
            Url::create_object_url_with_blob(
                &JsFuture::from(self.1.get_file())
                    .await
                    .map_err(|e| format!("Error getting file for opfs handle at [{}]: {}", self.0, jsstr(e)))?
                    .dyn_into::<Blob>()
                    .map_err(|e| format!("Opfs handle get_file at [{}] didn't return blob: {}", self.0, jsstr(e)))?,
            ).map_err(|e| format!("Error creating url from opfs file blob at [{}]: {}", self.0, jsstr(e)))?,
        );
    }
}

#[derive(Clone)]
pub struct OpfsWriteFile(pub DebugPath, pub FileSystemFileHandle);

impl OpfsWriteFile {
    pub async fn write_binary(&self, data: &[u8]) -> Result<(), String> {
        let w =
            FileSystemWritableFileStream::from(
                JsFuture::from(self.1.create_writable())
                    .await
                    .map_err(
                        |e| format!("Error getting file handle writable [{}]: {:?}", self.0, JSON::stringify(&e)),
                    )?,
            );
        JsFuture::from(
            w
                .write_with_u8_array(data)
                .map_err(|e| format!("Error writing message to opfs file [{}]: {:?}", self.0, jsstr(e)))?,
        )
            .await
            .map_err(|e| format!("Error writing message to opfs file [{}] (2): {:?}", self.0, jsstr(e)))?;
        JsFuture::from(w.close())
            .await
            .map_err(|e| format!("Error closing opfs file [{}] (2): {:?}", self.0, jsstr(e)))?;
        return Ok(());
    }

    pub async fn write_json<T: Serialize>(&self, data: &T) -> Result<(), String> {
        return self.write_binary(&serde_json::to_vec(data).unwrap()).await;
    }
}

#[derive(Clone)]
pub struct OpfsAmbig(pub DebugPath, pub JsValue);

impl OpfsAmbig {
    pub fn file(&self) -> Result<OpfsFile, String> {
        return Ok(
            OpfsFile(
                self.0.clone(),
                self
                    .1
                    .dyn_ref::<FileSystemFileHandle>()
                    .ok_or_else(|| format!("Opfs dir list entry at [{}], wanted file but was not", self.0))?
                    .clone(),
            ),
        );
    }

    pub fn dir(&self) -> Result<OpfsDir, String> {
        return Ok(
            OpfsDir(
                self.0.clone(),
                self
                    .1
                    .dyn_ref::<FileSystemDirectoryHandle>()
                    .ok_or_else(|| format!("Opfs dir list entry at [{}], wanted dir but was not", self.0))?
                    .clone(),
            ),
        );
    }
}

pub async fn opfs_root() -> OpfsDir {
    return OpfsDir(
        DebugPath(None),
        JsFuture::from(window().navigator().storage().get_directory())
            .await
            .expect("Error getting opfs root")
            .dyn_into::<FileSystemDirectoryHandle>()
            .expect("Couldn't get opfs root"),
    );
}
