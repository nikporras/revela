//! Revela: el puente entre la interfaz móvil (TypeScript, en un Web Worker) y el motor de
//! PhotoCraft. Toda la lógica vive aquí en Rust normal, probable con `cargo test`; `wasm.rs` solo
//! la expone a JavaScript.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod metadata;
#[cfg(target_arch = "wasm32")]
mod wasm;

use std::fmt;
use std::sync::Arc;

use photocraft_doc::Document;
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

    /// La copia de vista previa, reducida a `max_side` píxeles en su lado mayor si hace falta.
    pub fn preview(&self, max_side: u32) -> Result<Preview> {
        let doc = self.preview.clone().ok_or_else(|| Error("no hay ninguna foto abierta".into()))?;
        let img = photocraft_compose::thumbnail(&doc, max_side.max(1));
        Ok(Preview { width: img.width, height: img.height, pixels: img.pixels })
    }

    /// Codifica la foto a resolución de trabajo completa.
    pub fn export(&self, settings: ExportSettings) -> Result<Vec<u8>> {
        let doc = self.doc()?;
        let mut doc = (*doc).clone();
        doc.metadata = metadata::for_export(&doc.metadata, settings.keep_location);
        let mut opts = photocraft_io::ExportOptions::default();
        opts.encode.jpeg_quality = settings.quality.clamp(1, 100);
        // El XMP puede repetir la ubicación y el historial de edición; Revela no lo necesita.
        opts.xmp = photocraft_io::XmpEmbed::None;
        let r = photocraft_io::export(&doc, settings.format.extension(), &opts).map_err(|e| Error(format!("no se pudo exportar: {e}")))?;
        Ok(r.bytes)
    }
}

/// `doc` remuestreado a `w`×`h` por el motor (`image.imageSize` con el método `resample`).
fn resized(doc: Document, w: u32, h: u32, resample: &str) -> Result<Document> {
    let mut s = Session::new();
    s.add_document(doc, None);
    s.execute("image.imageSize", json!({ "width": w, "height": h, "resample": resample })).map_err(|e| Error(format!("no se pudo cambiar el tamaño de la foto: {e}")))?;
    let d = s.close(0).ok_or_else(|| Error("no se pudo cambiar el tamaño de la foto".into()))?;
    Ok(Arc::unwrap_or_clone(d.doc))
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
