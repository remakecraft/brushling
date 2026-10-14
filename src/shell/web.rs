//! Browser file access for the web build. The app's own Open and Save
//! dialogs work against an in-memory file store (see `App::put_file`); this
//! module moves files between that store and the user's disk: a file picker
//! for uploads, and a download for every finished save.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Where uploaded files appear in the app's file store.
pub const UPLOAD_DIR: &str = "/uploads";

/// One picked file, (name, bytes), or why it could not be read.
pub type Upload = Result<(String, Vec<u8>), String>;

/// Files the user picked, waiting for the next frame.
#[derive(Default, Clone)]
pub struct Uploads {
    inbox: Rc<RefCell<Vec<Upload>>>,
}

impl Uploads {
    /// Open the browser's file picker. Browsers allow this only shortly
    /// after a user gesture, which the Upload button click is.
    pub fn pick(&self) -> Result<(), String> {
        let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
        let input: web_sys::HtmlInputElement = document.create_element("input").map_err(js_err)?.dyn_into().map_err(|_| "not an input")?;
        input.set_type("file");
        input.set_accept(".bmp,.dib,image/bmp");
        let inbox = self.inbox.clone();
        let picked = input.clone();
        let on_change = Closure::once_into_js(move || {
            let Some(file) = picked.files().and_then(|f| f.get(0)) else { return };
            wasm_bindgen_futures::spawn_local(async move {
                let name = file.name();
                let read = JsFuture::from(file.array_buffer()).await;
                let entry = read.map(|buf| (name, js_sys::Uint8Array::new(&buf).to_vec())).map_err(js_err);
                inbox.borrow_mut().push(entry);
            });
        });
        input.set_onchange(Some(on_change.unchecked_ref()));
        input.click();
        Ok(())
    }

    /// Uploads that finished reading since the last call.
    pub fn take(&self) -> Vec<Upload> {
        std::mem::take(&mut *self.inbox.borrow_mut())
    }
}

/// Offer `bytes` to the user as a download named `name`.
pub fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let props = web_sys::BlobPropertyBag::new();
    props.set_type("image/bmp");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &props).map_err(js_err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_err)?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(js_err)?.dyn_into().map_err(|_| "not an anchor")?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Revoke once the browser has had time to start the download.
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 30_000).map_err(js_err)?;
    Ok(())
}

thread_local! {
    static UNSAVED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Tell the page whether closing it would lose changes: `index.html` asks
/// before unloading while `window.brushlingUnsaved` is true.
pub fn set_unsaved(unsaved: bool) {
    if UNSAVED.get() == Some(unsaved) {
        return;
    }
    if let Some(w) = web_sys::window() {
        if js_sys::Reflect::set(&w, &JsValue::from_str("brushlingUnsaved"), &JsValue::from_bool(unsaved)).is_ok() {
            UNSAVED.set(Some(unsaved));
        }
    }
}

/// The browser ignores egui's title command; set the tab title directly.
pub fn set_title(title: &str) {
    if let Some(d) = web_sys::window().and_then(|w| w.document()) {
        d.set_title(title);
    }
}

/// The file name part of a store path, for the download.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().filter(|n| !n.is_empty()).unwrap_or("untitled.bmp")
}

fn js_err(v: JsValue) -> String {
    v.as_string().or_else(|| v.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message()))).unwrap_or_else(|| format!("{v:?}"))
}
