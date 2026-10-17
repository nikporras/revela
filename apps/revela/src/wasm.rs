//! La API que ve JavaScript (el Web Worker de la interfaz). Solo traduce tipos: la lógica está en
//! `lib.rs`.

use wasm_bindgen::prelude::*;

use crate::{Editor, ExportFormat, ExportSettings};

fn js_err(e: crate::Error) -> JsError {
    JsError::new(&e.0)
}

#[wasm_bindgen]
pub struct Revela {
    editor: Editor,
}

#[wasm_bindgen]
impl Revela {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Revela {
        Revela { editor: Editor::new() }
    }

    /// Abre una foto y prepara su copia de vista previa de `preview_side` px. Devuelve `{ width, height, originalWidth, originalHeight, format, warnings, takenAt }`.
    pub fn open(&mut self, name: &str, bytes: &[u8], max_megapixels: f64, preview_side: u32) -> Result<JsValue, JsError> {
        let info = self.editor.open(name, bytes, max_megapixels, preview_side).map_err(js_err)?;
        let o = js_sys::Object::new();
        let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
        let warnings: js_sys::Array = info.warnings.iter().map(|w| JsValue::from_str(w)).collect();
        set("width", info.width.into())
            .and(set("height", info.height.into()))
            .and(set("originalWidth", info.original_width.into()))
            .and(set("originalHeight", info.original_height.into()))
            .and(set("format", JsValue::from_str(&info.format)))
            .and(set("warnings", warnings.into()))
            .and(set("takenAt", info.taken_at.as_deref().map_or(JsValue::NULL, JsValue::from_str)))
            .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
        Ok(o.into())
    }

    /// Vista previa RGBA8. Devuelve `{ width, height, pixels: Uint8ClampedArray }`.
    pub fn preview(&self, max_side: u32) -> Result<JsValue, JsError> {
        let p = self.editor.preview(max_side).map_err(js_err)?;
        let o = js_sys::Object::new();
        let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
        let pixels = js_sys::Uint8ClampedArray::from(p.pixels.as_slice());
        set("width", p.width.into())
            .and(set("height", p.height.into()))
            .and(set("pixels", pixels.into()))
            .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
        Ok(o.into())
    }

    /// Exporta a resolución completa: `format` "jpg" o "png", `quality` 1..100.
    pub fn export(&self, format: &str, quality: u8, keep_location: bool) -> Result<Vec<u8>, JsError> {
        let format = ExportFormat::parse(format).map_err(js_err)?;
        self.editor.export(ExportSettings { format, quality, keep_location }).map_err(js_err)
    }
}

impl Default for Revela {
    fn default() -> Self {
        Self::new()
    }
}

/// Bytes que ocupa ahora la memoria del wasm. Solo crece, así que también es el pico.
#[wasm_bindgen(js_name = memoryBytes)]
pub fn memory_bytes() -> f64 {
    wasm_bindgen::memory().dyn_into::<js_sys::WebAssembly::Memory>().map(|m| m.buffer().dyn_into::<js_sys::ArrayBuffer>().map(|b| b.byte_length() as f64).unwrap_or(0.0)).unwrap_or(0.0)
}
