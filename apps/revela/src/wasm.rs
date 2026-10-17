//! La API que ve JavaScript (el Web Worker de la interfaz). Solo traduce tipos: la lógica está en
//! `lib.rs`.

use wasm_bindgen::prelude::*;

use crate::geometria::{Geometria, angulo_de_linea};
use crate::{Editor, ExportFormat, ExportSettings, Preview, ajustes};

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

    /// Vista previa RGBA8 sin ajustes. Devuelve `{ width, height, pixels: Uint8ClampedArray }`.
    pub fn preview(&self, max_side: u32) -> Result<JsValue, JsError> {
        preview_js(self.editor.preview(max_side).map_err(js_err)?)
    }

    /// La previa con los ajustes y la geometría aplicados por el motor (para compararla con la GPU).
    #[wasm_bindgen(js_name = developPreview)]
    pub fn develop_preview(&self, max_side: u32, ajustes_json: &str, geometria_json: &str) -> Result<JsValue, JsError> {
        let a = ajustes::sanear(ajustes_json).map_err(js_err)?;
        let g = Geometria::de_json(geometria_json).map_err(js_err)?;
        preview_js(self.editor.develop_preview(max_side, &a, &g).map_err(js_err)?)
    }

    /// Tamaño `[ancho, alto]` de la foto exportada con esta geometría.
    #[wasm_bindgen(js_name = exportSize)]
    pub fn export_size(&self, geometria_json: &str) -> Result<Vec<u32>, JsError> {
        let g = Geometria::de_json(geometria_json).map_err(js_err)?;
        let (w, h) = self.editor.export_size(&g).map_err(js_err)?;
        Ok(vec![w, h])
    }

    /// Las filas `fila0..fila1` de una ventana `[x0, y0, x1, y1]` (fracciones) del encuadre a
    /// resolución completa, dibujada a como mucho `ancho`×`alto`, sin ajustes: la previa nítida
    /// con zoom. Devuelve `{ ancho, alto, fila0, pixels: Uint8ClampedArray }`.
    #[allow(clippy::too_many_arguments)]
    pub fn detalle(&self, geometria_json: &str, x0: f64, y0: f64, x1: f64, y1: f64, ancho: u32, alto: u32, fila0: u32, fila1: u32) -> Result<JsValue, JsError> {
        let g = Geometria::de_json(geometria_json).map_err(js_err)?;
        let f = self.editor.detalle(&g, [x0, y0, x1, y1], ancho, alto, fila0..fila1).map_err(js_err)?;
        let o = js_sys::Object::new();
        let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
        set("ancho", f.ancho.into())
            .and(set("alto", f.alto.into()))
            .and(set("fila0", f.fila0.into()))
            .and(set("pixels", js_sys::Uint8ClampedArray::from(f.pixels.as_slice()).into()))
            .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
        Ok(o.into())
    }

    /// Exporta a resolución completa con ajustes y geometría: `format` "jpg" o "png", `quality`
    /// 1..100. `progreso` recibe la fracción hecha (0..1) mientras trabaja.
    pub fn export(&self, format: &str, quality: u8, keep_location: bool, ajustes_json: &str, geometria_json: &str, progreso: &js_sys::Function) -> Result<Vec<u8>, JsError> {
        let format = ExportFormat::parse(format).map_err(js_err)?;
        let a = ajustes::sanear(ajustes_json).map_err(js_err)?;
        let g = Geometria::de_json(geometria_json).map_err(js_err)?;
        let avisar = |f: f32| {
            let _ = progreso.call1(&JsValue::NULL, &JsValue::from_f64(f64::from(f)));
        };
        self.editor.export(ExportSettings { format, quality, keep_location }, &a, &g, &avisar).map_err(js_err)
    }
}

/// El plano de una geometría para una foto de `w`×`h`: `{ ancho, alto, m: Float32Array(6) }`
/// (ver `geometria::Plano`). La GPU lo usa tal cual, con los mismos `f32`.
#[wasm_bindgen]
pub fn plano(geometria_json: &str, w: u32, h: u32) -> Result<JsValue, JsError> {
    let p = Geometria::de_json(geometria_json).map_err(js_err)?.plano(w, h);
    let o = js_sys::Object::new();
    let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
    set("ancho", p.ancho.into())
        .and(set("alto", p.alto.into()))
        .and(set("m", js_sys::Float32Array::from(&p.m[..]).into()))
        .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
    Ok(o.into())
}

/// Ángulo (grados, en el sentido del reloj) que deja horizontal o vertical la línea dibujada.
#[wasm_bindgen(js_name = anguloLinea)]
pub fn angulo_linea(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    angulo_de_linea((x0, y0), (x1, y1))
}

fn preview_js(p: Preview) -> Result<JsValue, JsError> {
    let o = js_sys::Object::new();
    let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
    let pixels = js_sys::Uint8ClampedArray::from(p.pixels.as_slice());
    set("width", p.width.into())
        .and(set("height", p.height.into()))
        .and(set("pixels", pixels.into()))
        .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
    Ok(o.into())
}

/// Los parámetros del shader para unos ajustes (ver `ajustes::Gpu`). Los números van en un
/// `Float32Array` para que la GPU reciba exactamente los `f32` que usa el motor.
#[wasm_bindgen(js_name = gpuParams)]
pub fn gpu_params(ajustes_json: &str) -> Result<JsValue, JsError> {
    let g = ajustes::Gpu::de(&ajustes::sanear(ajustes_json).map_err(js_err)?);
    let o = js_sys::Object::new();
    let set = |k: &str, v: JsValue| js_sys::Reflect::set(&o, &JsValue::from_str(k), &v).map(|_| ());
    let f32s = |v: &[f32]| JsValue::from(js_sys::Float32Array::from(v));
    let bools = |v: &[bool]| JsValue::from(v.iter().map(|b| JsValue::from_bool(*b)).collect::<js_sys::Array>());
    let tints: Vec<f32> = g.tints.iter().flatten().copied().collect();
    set("wb", g.wb.into())
        .and(set("gain", f32s(&g.gain)))
        .and(set("contrastK", f32s(&[g.contrast_k])))
        .and(set("color", g.color.into()))
        .and(set("satVib", f32s(&[g.saturation, g.vibrance])))
        .and(set("lutOn", bools(&g.lut_on)))
        .and(set("luts", f32s(&g.luts)))
        .and(set("hsl", g.hsl.into()))
        .and(set("hslHue", f32s(&g.hsl_hue)))
        .and(set("hslSat", f32s(&g.hsl_sat)))
        .and(set("hslLum", f32s(&g.hsl_lum)))
        .and(set("grade", g.grade.into()))
        .and(set("tints", f32s(&tints)))
        .and(set("lums", f32s(&g.lums)))
        .and(set("blendBalance", f32s(&[g.blend, g.balance])))
        .map_err(|_| JsError::new("no se pudo preparar la respuesta"))?;
    Ok(o.into())
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
