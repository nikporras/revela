//! Revela: el puente entre la interfaz móvil (TypeScript, en un Web Worker) y el motor de
//! PhotoCraft. Toda la lógica vive aquí en Rust normal, probable con `cargo test`; `wasm.rs` solo
//! la expone a JavaScript.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod ajustes;
pub mod efectos;
pub mod geometria;
pub mod metadata;
#[cfg(target_arch = "wasm32")]
mod wasm;

use std::fmt;
use std::sync::Arc;

use ajustes::Revelado;
use photocraft_algo::camera_raw::develop;
use photocraft_doc::Document;
use geometria::{Geometria, bilineal};
use photocraft_engine::Session;
use serde_json::json;

/// Error con un mensaje en español que la interfaz puede mostrar tal cual.
#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Datos de la foto abierta.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoInfo {
    /// Tamaño de trabajo (después de aplicar el límite de megapíxeles).
    pub width: u32,
    pub height: u32,
    /// Tamaño original del archivo, ya enderezado según su orientación EXIF.
    pub original_width: u32,
    pub original_height: u32,
    /// Formato detectado por su contenido ("jpeg", "heif", "png"…), no por el nombre.
    pub format: String,
    /// Avisos del motor al abrir (cosas aproximadas o ignoradas).
    pub warnings: Vec<String>,
    /// Fecha y hora de captura del EXIF como `AAAAMMDD_HHMMSS` (el nombre que pone la cámara de
    /// Samsung), para nombrar la exportación cuando el navegador entrega un nombre sin sentido.
    pub taken_at: Option<String>,
}

impl PhotoInfo {
    pub fn downscaled(&self) -> bool {
        self.width != self.original_width || self.height != self.original_height
    }
}

/// Imagen RGBA8 (alfa directo) lista para un `<canvas>`.
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Unas filas de la zona nítida del zoom ([`Editor::detalle`]): la ventana entera mide
/// `ancho`×`alto`; `pixels` son RGBA8 de las filas desde `fila0`.
pub struct Franja {
    pub ancho: u32,
    pub alto: u32,
    pub fila0: u32,
    pub pixels: Vec<u8>,
}

/// Formatos de exportación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Jpeg,
    Png,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            other => Err(Error(format!("formato de exportación no soportado: {other}"))),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

/// Opciones de exportación.
#[derive(Debug, Clone, Copy)]
pub struct ExportSettings {
    pub format: ExportFormat,
    /// Calidad JPG 1..=100 (se ignora en PNG).
    pub quality: u8,
    /// Conservar la ubicación GPS. Desactivado por defecto: se borra.
    pub keep_location: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self { format: ExportFormat::Jpeg, quality: 90, keep_location: false }
    }
}

/// El editor: una sesión del motor con la foto abierta a resolución de trabajo.
#[derive(Default)]
pub struct Editor {
    /// La foto a resolución de trabajo: la que se exporta.
    session: Session,
    /// La misma foto reducida al tamaño de la pantalla: la que se dibuja.
    preview: Option<Arc<Document>>,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Abre una foto. Si pasa de `max_megapixels`, se reduce a ese límite antes de editar. La
    /// orientación EXIF ya la aplica el decodificador del motor: la foto queda derecha.
    ///
    /// Además prepara la **copia de vista previa** (`preview_side` píxeles en el lado mayor), que
    /// es la que se dibuja en pantalla: renderizar la foto completa en cada cambio sería lento y
    /// gastaría mucha memoria.
    pub fn open(&mut self, name: &str, bytes: &[u8], max_megapixels: f64, preview_side: u32) -> Result<PhotoInfo> {
        let format = photocraft_codecs::detect(bytes).map(|f| format!("{f:?}").to_ascii_lowercase()).unwrap_or_else(|| "desconocido".into());
        // Una sola foto a la vez: se suelta la anterior antes de decodificar la nueva.
        self.session = Session::new();
        self.preview = None;
        let imported = photocraft_io::import(name, bytes).map_err(|e| Error(format!("no se pudo abrir la foto: {e}")))?;
        let (ow, oh) = (imported.document.size.width, imported.document.size.height);
        let mut doc = imported.document;
        // Los ajustes trabajan en RGB de 8 bits: una foto en gris, CMYK, Lab o de 16 bits se
        // convierte una vez al abrir (los teléfonos ya entregan RGB de 8 bits).
        if doc.mode != photocraft_doc::ColorMode::Rgb {
            doc = with_command(doc, "image.mode.rgb", json!({}))?;
        }
        if doc.depth != photocraft_doc::SampleType::U8 {
            doc = with_command(doc, "image.mode.bits8", json!({}))?;
        }
        if let Some((w, h)) = fit_megapixels(ow, oh, max_megapixels) {
            // Esta reducción es la que se exporta: bicúbico (el motor elige el más nítido).
            doc = resized(doc, w, h, "bicubicAutomatic")?;
        }
        let (w, h) = (doc.size.width, doc.size.height);
        let preview = match fit_side(w, h, preview_side) {
            // Solo para la pantalla: bilineal, que el motor ensancha al reducir (sin dentado) y
            // en wasm tarda la tercera parte que el bicúbico.
            Some((pw, ph)) => resized(doc.clone(), pw, ph, "bilinear")?,
            None => doc.clone(),
        };
        self.session.add_document(doc, None);
        self.preview = Some(Arc::new(preview));
        let taken_at = self.doc()?.metadata.exif.as_deref().and_then(|e| metadata::taken_at(e));
        Ok(PhotoInfo { width: w, height: h, original_width: ow, original_height: oh, format, warnings: imported.warnings, taken_at })
    }

    fn doc(&self) -> Result<Arc<Document>> {
        self.session.active().map(|d| d.doc.clone()).ok_or_else(|| Error("no hay ninguna foto abierta".into()))
    }

    /// La copia de vista previa sin ajustes, reducida a `max_side` píxeles en su lado mayor si
    /// hace falta. Es la textura de la que parte el shader.
    pub fn preview(&self, max_side: u32) -> Result<Preview> {
        let doc = self.preview.clone().ok_or_else(|| Error("no hay ninguna foto abierta".into()))?;
        let img = photocraft_compose::thumbnail(&doc, max_side.max(1));
        Ok(Preview { width: img.width, height: img.height, pixels: img.pixels })
    }

    /// La misma previa con los ajustes aplicados **por el motor** (lo que hará la exportación).
    /// Sirve para comprobar que el shader de la GPU da lo mismo.
    pub fn develop_preview(&self, max_side: u32, ajustes: &Revelado, geom: &Geometria) -> Result<Preview> {
        let base = self.preview(max_side)?;
        let (bw, bh) = (base.width, base.height);
        let plano = geom.plano(bw, bh);
        let (w, h) = (plano.ancho as usize, plano.alto as usize);
        let texel = |x: i32, y: i32| -> [f32; 4] {
            let i = (y as usize * bw as usize + x as usize) * 4;
            base.pixels.get(i..i + 4).map_or([0.0; 4], |q| [q[0], q[1], q[2], q[3]].map(|v| f32::from(v) / 255.0))
        };
        let mut px: Vec<[f32; 4]> = if geom.es_identidad() {
            base.pixels.as_chunks::<4>().0.iter().map(|q| q.map(|v| f32::from(v) / 255.0)).collect()
        } else {
            (0..plano.alto)
                .flat_map(|v| (0..plano.ancho).map(move |u| (u, v)))
                .map(|(u, v)| {
                    let (x, y) = plano.origen_de(u, v);
                    bilineal(x, y, bw, bh, texel)
                })
                .collect()
        };
        // Sin recortar a 0..1 hasta después de la viñeta, como el motor (`to_u8` recorta).
        develop(&mut px, w, h, &ajustes.raw, true);
        efectos::aplicar_puntuales(&mut px, &ajustes.puntuales);
        if ajustes.vineta.activa() {
            ajustes.vineta.plano(plano.ancho, plano.alto).aplicar(&mut px, w, 0, 0);
        }
        let pixels = px.iter().flat_map(|q| [to_u8(q[0]), to_u8(q[1]), to_u8(q[2]), to_u8(q[3])]).collect();
        Ok(Preview { width: plano.ancho, height: plano.alto, pixels })
    }

    /// Tamaño de la foto exportada con esta geometría.
    pub fn export_size(&self, geom: &Geometria) -> Result<(u32, u32)> {
        let doc = self.doc()?;
        let p = geom.plano(doc.size.width, doc.size.height);
        Ok((p.ancho, p.alto))
    }

    /// Una ventana del encuadre a resolución completa, para ver nítida la previa con zoom:
    /// `ventana` es `[x0, y0, x1, y1]` en fracciones de la salida y se dibuja a `ancho`×`alto`
    /// (como mucho, la resolución de la foto). Sin ajustes: los pone la GPU, así que mover un
    /// slider no obliga a pedirla otra vez.
    ///
    /// Devuelve solo las filas `filas` de esa ventana: la interfaz la pide **por franjas** para
    /// que, entre una y otra, el worker atienda antes a los sliders (y pueda dejarla a medias).
    pub fn detalle(&self, geom: &Geometria, ventana: [f64; 4], ancho: u32, alto: u32, filas: std::ops::Range<u32>) -> Result<Franja> {
        let doc = self.doc()?;
        let [x0, y0, x1, y1] = ventana.map(|v| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 });
        if x1 <= x0 || y1 <= y0 {
            return Err(Error("ventana vacía".into()));
        }
        let completo = geom.plano(doc.size.width, doc.size.height);
        // Nunca más píxeles que los de la foto en esa ventana (ni más de 4096 de lado). Es
        // determinista: todas las franjas de una ventana calculan el mismo tamaño.
        let tope = |n: u32, f: f64, lado: u32| n.min((f * f64::from(lado)).ceil() as u32).clamp(1, 4096);
        let plano = completo.ventana([x0, y0, x1, y1], tope(ancho, x1 - x0, completo.ancho), tope(alto, y1 - y0, completo.alto));
        let (v0, v1) = (filas.start.min(plano.alto), filas.end.min(plano.alto));
        if v0 >= v1 {
            return Err(Error("franja vacía".into()));
        }
        let franja = plano.filas(v0, v1);
        let (w, h) = (franja.ancho as usize, franja.alto as usize);
        let mut pixels = vec![0u8; w * h * 4];
        por_bloques(&doc, &franja, |px, u0, v0, tw, _| {
            for (k, q) in px.iter().enumerate() {
                let (x, y) = (u0 as usize + k % tw, v0 as usize + k / tw);
                let i = (y * w + x) * 4;
                if let Some(o) = pixels.get_mut(i..i + 4) {
                    o.copy_from_slice(&q.map(to_u8));
                }
            }
            Ok(())
        })?;
        Ok(Franja { ancho: plano.ancho, alto: plano.alto, fila0: v0, pixels })
    }

    /// Codifica la foto a resolución de trabajo completa con los ajustes. El motor compone la
    /// foto **por franjas** y revela cada una por separado (los ajustes son por píxel, así que no
    /// hay costuras): la memoria no pasa de la foto, su salida de 8 bits y una franja.
    pub fn export(&self, settings: ExportSettings, ajustes: &Revelado, geom: &Geometria, progreso: &dyn Fn(f32)) -> Result<Vec<u8>> {
        let doc = self.doc()?;
        let plano = geom.plano(doc.size.width, doc.size.height);
        let (w, h) = (plano.ancho as usize, plano.alto as usize);
        // Una pasada a RGB (lo transparente, sobre blanco). Solo un PNG con transparencia necesita
        // una segunda pasada con alfa: así una foto normal ocupa 3 bytes por píxel y no 4 + 3.
        // El revelado es el 90 % del trabajo; codificar, el resto.
        let (rgb, transparente) = revelar(&doc, ajustes, geom, 3, &|f| progreso(f * 0.9))?;
        let alpha = transparente && settings.format == ExportFormat::Png;
        let data = if alpha {
            drop(rgb);
            revelar(&doc, ajustes, geom, 4, &|f| progreso(0.45 + f * 0.45))?.0
        } else {
            rgb
        };
        let meta = metadata::for_export(&doc.metadata, settings.keep_location);
        let codec_meta = photocraft_codecs::Metadata {
            // El códec quiere el bloque TIFF sin el prefijo "Exif\0\0" de JPEG.
            exif: meta.exif.map(|e| e.strip_prefix(b"Exif\0\0").unwrap_or(&e).to_vec()),
            dpi: Some((doc.resolution_dpi, doc.resolution_dpi)),
            ..Default::default()
        };
        let layout = if alpha { photocraft_codecs::ChannelLayout::Rgba } else { photocraft_codecs::ChannelLayout::Rgb };
        let img = photocraft_codecs::Image::from_raw(w as u32, h as u32, layout, photocraft_codecs::SampleType::U8, data)
            .map_err(|e| Error(format!("no se pudo exportar: {e}")))?
            .with_icc(doc.icc_profile.as_ref().map(|i| i.to_vec()))
            .with_meta(codec_meta);
        let format = match settings.format {
            ExportFormat::Jpeg => photocraft_codecs::Format::Jpeg,
            ExportFormat::Png => photocraft_codecs::Format::Png,
        };
        let opts = photocraft_codecs::EncodeOptions { jpeg_quality: settings.quality.clamp(1, 100), ..Default::default() };
        let bytes = photocraft_codecs::encode(&img, format, &opts).map_err(|e| Error(format!("no se pudo exportar: {e}")))?;
        progreso(1.0);
        Ok(bytes)
    }
}

/// Filas por franja al exportar: poca memoria (una franja de 4000 px de ancho son ~16 MB en f32).
const FILAS_FRANJA: i32 = 256;

/// Lado de los bloques al exportar con geometría.
const BLOQUE: u32 = 256;

/// La foto revelada con `ajustes` y encuadrada con `geom`, en 8 bits con `canales` 3 (RGB, con lo
/// transparente sobre blanco) o 4 (RGBA). Devuelve también si había transparencia.
///
/// Sin geometría, el motor compone la foto por franjas. Con geometría, la salida se recorre por
/// bloques: cada uno pide al motor solo la zona de la foto que cubre ([`geometria::Plano::huella`])
/// y la remuestrea con el mismo bilineal que la GPU. La memoria no pasa de la salida y un bloque.
fn revelar(doc: &Document, ajustes: &Revelado, geom: &Geometria, canales: usize, progreso: &dyn Fn(f32)) -> Result<(Vec<u8>, bool)> {
    let plano = geom.plano(doc.size.width, doc.size.height);
    // La viñeta, con coordenadas del encuadre entero (no de cada franja o bloque).
    let vineta = ajustes.vineta.activa().then(|| ajustes.vineta.plano(plano.ancho, plano.alto));
    let (w, h) = (plano.ancho as usize, plano.alto as usize);
    let canvas = doc.bounds();
    let len = w.checked_mul(h).and_then(|n| n.checked_mul(canales)).ok_or_else(|| Error("foto demasiado grande".into()))?;
    let mut out = Vec::new();
    out.try_reserve_exact(len).map_err(|_| Error("no hay memoria para exportar la foto".into()))?;
    out.resize(len, 0u8);
    let mut transparente = false;
    let mut escribir = |px: &[[f32; 4]], x0: usize, y0: usize, bw: usize| -> Result<()> {
        for (i, q) in px.iter().enumerate() {
            let (x, y) = (x0 + i % bw, y0 + i / bw);
            let o = out.get_mut((y * w + x) * canales..(y * w + x + 1) * canales).ok_or_else(|| Error("bloque fuera de la foto".into()))?;
            let a = to_u8(q[3]);
            transparente |= a < 255;
            if canales == 4 {
                o.copy_from_slice(&[to_u8(q[0]), to_u8(q[1]), to_u8(q[2]), a]);
            } else {
                let a = u16::from(a);
                for c in 0..3 {
                    o[c] = ((u16::from(to_u8(q[c])) * a + 255 * (255 - a) + 127) / 255) as u8;
                }
            }
        }
        Ok(())
    };
    if geom.es_identidad() {
        photocraft_compose::render_bands(doc, canvas, FILAS_FRANJA, |mut band| -> Result<()> {
            let y0 = usize::try_from(band.rect.y0 - canvas.y0).map_err(|_| Error("franja fuera de la foto".into()))?;
            let n = band.rect.height() as usize;
            develop(&mut band.px, w, n, &ajustes.raw, true);
            efectos::aplicar_puntuales(&mut band.px, &ajustes.puntuales);
            if let Some(v) = &vineta {
                v.aplicar(&mut band.px, w, 0, y0);
            }
            escribir(&band.px, 0, y0, w)?;
            progreso((y0 + n) as f32 / h.max(1) as f32);
            Ok(())
        })?;
        return Ok((out, transparente));
    }
    let total = (plano.ancho.div_ceil(BLOQUE) * plano.alto.div_ceil(BLOQUE)).max(1) as f32;
    let mut hechos = 0.0;
    por_bloques(doc, &plano, |px, u0, v0, tw, th| {
        develop(px, tw, th, &ajustes.raw, true);
        efectos::aplicar_puntuales(px, &ajustes.puntuales);
        if let Some(v) = &vineta {
            v.aplicar(px, tw, u0 as usize, v0 as usize);
        }
        escribir(px, u0 as usize, v0 as usize, tw)?;
        hechos += 1.0;
        progreso(hechos / total);
        Ok(())
    })?;
    Ok((out, transparente))
}

/// Recorre la salida de `plano` por bloques de [`BLOQUE`] px: cada bloque pide al motor solo la
/// zona de la foto que cubre ([`geometria::Plano::huella`]) y la remuestrea con el mismo bilineal
/// que la GPU. `f` recibe los píxeles del bloque (f32, sin ajustes), su esquina y su tamaño.
fn por_bloques(doc: &Document, plano: &geometria::Plano, mut f: impl FnMut(&mut [[f32; 4]], u32, u32, usize, usize) -> Result<()>) -> Result<()> {
    let canvas = doc.bounds();
    let (bx, by) = (plano.ancho.div_ceil(BLOQUE), plano.alto.div_ceil(BLOQUE));
    let (sw, sh) = plano.origen;
    for j in 0..by {
        for i in 0..bx {
            let (u0, v0) = (i * BLOQUE, j * BLOQUE);
            let (u1, v1) = ((u0 + BLOQUE).min(plano.ancho), (v0 + BLOQUE).min(plano.alto));
            let (tw, th) = ((u1 - u0) as usize, (v1 - v0) as usize);
            let mut px = vec![[0.0f32; 4]; tw * th];
            if let Some([x0, y0, x1, y1]) = plano.huella(u0, v0, u1, v1) {
                let zona = photocraft_compose::render(doc, photocraft_geom::Rect::new(canvas.x0 + x0, canvas.y0 + y0, canvas.x0 + x1, canvas.y0 + y1));
                let zw = (x1 - x0) as usize;
                let texel = |x: i32, y: i32| -> [f32; 4] {
                    if x < x0 || y < y0 || x >= x1 || y >= y1 {
                        return [0.0; 4];
                    }
                    zona.px.get((y - y0) as usize * zw + (x - x0) as usize).copied().unwrap_or([0.0; 4])
                };
                for (k, q) in px.iter_mut().enumerate() {
                    let (u, v) = (u0 + (k % tw) as u32, v0 + (k / tw) as u32);
                    let (x, y) = plano.origen_de(u, v);
                    *q = bilineal(x, y, sw, sh, texel);
                }
            }
            f(&mut px, u0, v0, tw, th)?;
        }
    }
    Ok(())
}

/// 0..1 → 0..255, redondeando (como la GPU al escribir un RGBA8).
fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// `doc` después de un comando del motor que no necesita historial.
fn with_command(doc: Document, id: &str, params: serde_json::Value) -> Result<Document> {
    let mut s = Session::new();
    s.add_document(doc, None);
    s.execute(id, params).map_err(|e| Error(format!("no se pudo preparar la foto ({id}): {e}")))?;
    let d = s.close(0).ok_or_else(|| Error(format!("no se pudo preparar la foto ({id})")))?;
    Ok(Arc::unwrap_or_clone(d.doc))
}

/// `doc` remuestreado a `w`×`h` por el motor (`image.imageSize` con el método `resample`).
fn resized(doc: Document, w: u32, h: u32, resample: &str) -> Result<Document> {
    with_command(doc, "image.imageSize", json!({ "width": w, "height": h, "resample": resample }))
}

/// El tamaño cuyo lado mayor es `side`, o `None` si ya cabe.
pub fn fit_side(w: u32, h: u32, side: u32) -> Option<(u32, u32)> {
    let longest = w.max(h);
    if side == 0 || longest <= side {
        return None;
    }
    let k = f64::from(side) / f64::from(longest);
    Some((((f64::from(w) * k).round() as u32).max(1), ((f64::from(h) * k).round() as u32).max(1)))
}

/// El tamaño que cabe en `max_megapixels` manteniendo la proporción, o `None` si ya cabe.
pub fn fit_megapixels(w: u32, h: u32, max_megapixels: f64) -> Option<(u32, u32)> {
    let max_px = max_megapixels * 1_000_000.0;
    let area = f64::from(w) * f64::from(h);
    if !max_px.is_finite() || max_px <= 0.0 || area <= max_px {
        return None;
    }
    let k = (max_px / area).sqrt();
    let nw = ((f64::from(w) * k).floor() as u32).max(1);
    let nh = ((f64::from(h) * k).floor() as u32).max(1);
    Some((nw, nh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_keeps_small_photos() {
        assert_eq!(fit_megapixels(4000, 3000, 12.0), None);
        assert_eq!(fit_megapixels(4000, 3000, 0.0), None);
        assert_eq!(fit_megapixels(4000, 3000, f64::NAN), None);
    }

    #[test]
    fn fit_reduces_to_the_limit_keeping_the_ratio() {
        let (w, h) = fit_megapixels(8064, 6048, 24.0).unwrap_or_default();
        assert!(u64::from(w) * u64::from(h) <= 24_000_000);
        assert!((f64::from(w) / f64::from(h) - 8064.0 / 6048.0).abs() < 0.001);
    }

    #[test]
    fn fit_side_scales_the_longest_side() {
        assert_eq!(fit_side(4000, 3000, 2048), Some((2048, 1536)));
        assert_eq!(fit_side(3000, 4000, 2048), Some((1536, 2048)));
        assert_eq!(fit_side(1000, 800, 2048), None);
        assert_eq!(fit_side(1000, 800, 0), None);
    }

    #[test]
    fn export_format_parses() {
        assert_eq!(ExportFormat::parse("JPG").ok(), Some(ExportFormat::Jpeg));
        assert!(ExportFormat::parse("gif").is_err());
    }
}
