//! Abrir → exportar con un JPG real: orientación EXIF, límite de megapíxeles y borrado del GPS.

use photocraft_codecs::{ChannelLayout, EncodeOptions, Format, Image, SampleType};
use revela_core::{Editor, ExportFormat, ExportSettings};

/// EXIF little-endian: Orientation = 6 (girar 90° a la derecha) y una IFD GPS cuya latitud son
/// 24 bytes 0xAA, fáciles de buscar en el archivo exportado.
fn exif_with_gps() -> Vec<u8> {
    let mut b = b"Exif\0\0II*\0".to_vec();
    b.extend(8u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend([0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0]);
    b.extend([0x25, 0x88, 4, 0, 1, 0, 0, 0, 38, 0, 0, 0]);
    b.extend(0u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend([0x02, 0x00, 5, 0, 3, 0, 0, 0, 56, 0, 0, 0]);
    b.extend(0u32.to_le_bytes());
    b.extend([0xAA; 24]);
    b
}

/// Un JPG de 60×40 guardado "de lado", con el EXIF de arriba justo después de SOI.
fn camera_jpeg() -> Vec<u8> {
    let px: Vec<f32> = (0..40).flat_map(|y| (0..60).flat_map(move |x| [x as f32 / 59.0, y as f32 / 39.0, 0.5])).collect();
    let img = Image::from_normalized(60, 40, ChannelLayout::Rgb, SampleType::U8, &px).unwrap();
    let jpeg = photocraft_codecs::encode(&img, Format::Jpeg, &EncodeOptions::default()).unwrap();
    let seg = exif_with_gps();
    let mut out = jpeg[..2].to_vec();
    out.extend([0xFF, 0xE1]);
    out.extend(((seg.len() + 2) as u16).to_be_bytes());
    out.extend(&seg);
    out.extend(&jpeg[2..]);
    out
}

fn has_gps_bytes(b: &[u8]) -> bool {
    b.windows(8).any(|w| w == [0xAA; 8])
}

#[test]
fn opens_upright_and_exports_without_location() {
    let src = camera_jpeg();
    assert!(has_gps_bytes(&src));
    let mut ed = Editor::new();
    let info = ed.open("IMG_0001.JPG", &src, 24.0, 2048).unwrap();
    assert_eq!((info.width, info.height), (40, 60), "la orientación EXIF no se aplicó");
    assert_eq!(info.format, "jpeg");
    assert!(!info.downscaled());

    let out = ed.export(ExportSettings::default(), &Default::default()).unwrap();
    assert!(!has_gps_bytes(&out), "la ubicación GPS quedó en el JPG exportado");
    let back = photocraft_codecs::decode(&out).unwrap();
    assert_eq!((back.width(), back.height()), (40, 60));
    // El JPG exportado está derecho y no le pide al visor que lo gire otra vez.
    let keep = photocraft_codecs::DecodeOptions { keep_orientation: true, ..Default::default() };
    let stored = photocraft_codecs::decode_with(&out, &keep).unwrap();
    assert_eq!((stored.width(), stored.height()), (40, 60));
    assert_eq!(stored.meta.exif.as_deref().map_or(1, photocraft_codecs::exif_orientation), 1);
}

#[test]
fn keeps_location_only_when_asked() {
    let mut ed = Editor::new();
    ed.open("a.jpg", &camera_jpeg(), 24.0, 2048).unwrap();
    let out = ed.export(ExportSettings { keep_location: true, ..Default::default() }, &Default::default()).unwrap();
    assert!(has_gps_bytes(&out));
}

#[test]
fn png_export_has_no_location_either() {
    let mut ed = Editor::new();
    ed.open("a.jpg", &camera_jpeg(), 24.0, 2048).unwrap();
    let out = ed.export(ExportSettings { format: ExportFormat::Png, ..Default::default() }, &Default::default()).unwrap();
    assert!(out.starts_with(b"\x89PNG"));
    assert!(!has_gps_bytes(&out));
}

#[test]
fn megapixel_limit_reduces_the_photo() {
    let mut ed = Editor::new();
    // 40×60 = 2400 px; el límite de 600 px la deja en 20×30.
    let info = ed.open("a.jpg", &camera_jpeg(), 0.0006, 2048).unwrap();
    assert!(info.downscaled());
    assert_eq!((info.original_width, info.original_height), (40, 60));
    assert_eq!((info.width, info.height), (20, 30));
    let p = ed.preview(15).unwrap();
    assert_eq!((p.width, p.height), (10, 15));
    assert_eq!(p.pixels.len(), 10 * 15 * 4);
}

#[test]
fn preview_comes_from_the_reduced_copy_and_export_from_the_full_photo() {
    let mut ed = Editor::new();
    let info = ed.open("a.jpg", &camera_jpeg(), 24.0, 30).unwrap();
    assert_eq!((info.width, info.height), (40, 60));
    let p = ed.preview(2048).unwrap();
    assert_eq!((p.width, p.height), (20, 30));
    let out = photocraft_codecs::decode(&ed.export(ExportSettings::default(), &Default::default()).unwrap()).unwrap();
    assert_eq!((out.width(), out.height()), (40, 60));
}

#[test]
fn reports_the_capture_date_for_naming() {
    // Un JPG con solo DateTimeOriginal en su IFD EXIF, como el de la cámara de un teléfono.
    let mut t = b"Exif\0\0II*\0".to_vec();
    t.extend(8u32.to_le_bytes());
    t.extend(1u16.to_le_bytes());
    t.extend([0x69, 0x87, 4, 0, 1, 0, 0, 0, 26, 0, 0, 0]);
    t.extend(0u32.to_le_bytes());
    t.extend(1u16.to_le_bytes());
    t.extend([0x03, 0x90, 2, 0, 20, 0, 0, 0, 44, 0, 0, 0]);
    t.extend(0u32.to_le_bytes());
    t.extend(b"2026:10:11 01:18:47\0");
    let px = vec![0.5f32; 8 * 8 * 3];
    let img = Image::from_normalized(8, 8, ChannelLayout::Rgb, SampleType::U8, &px).unwrap();
    let jpeg = photocraft_codecs::encode(&img, Format::Jpeg, &EncodeOptions::default()).unwrap();
    let mut src = jpeg[..2].to_vec();
    src.extend([0xFF, 0xE1]);
    src.extend(((t.len() + 2) as u16).to_be_bytes());
    src.extend(&t);
    src.extend(&jpeg[2..]);
    let mut ed = Editor::new();
    let info = ed.open("1000205213.jpg", &src, 24.0, 2048).unwrap();
    assert_eq!(info.taken_at.as_deref(), Some("20261011_011847"));
    // Sin EXIF no hay fecha.
    assert_eq!(ed.open("x.jpg", &jpeg, 24.0, 2048).unwrap().taken_at, None);
}

/// Un PNG de `w`×`h` con un degradado de color, opcionalmente con una zona transparente.
fn png(w: u32, h: u32, transparent: bool) -> Vec<u8> {
    let layout = if transparent { ChannelLayout::Rgba } else { ChannelLayout::Rgb };
    let px: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let (fx, fy) = (x as f32 / (w - 1) as f32, y as f32 / (h - 1) as f32);
            let mut v = vec![0.2 + 0.7 * fx, 0.3 + 0.5 * fy, 0.8 - 0.6 * fx * fy];
            if transparent {
                v.push(if x < w / 4 { 0.0 } else { 1.0 });
            }
            v
        })
        .collect();
    let img = Image::from_normalized(w, h, layout, SampleType::U8, &px).unwrap();
    photocraft_codecs::encode(&img, Format::Png, &EncodeOptions::default()).unwrap()
}

#[test]
fn adjusted_export_matches_the_engines_camera_raw_filter() {
    let src = png(300, 200, false);
    let json = r#"{"temperature": 20, "tint": -10, "exposure": 0.3, "contrast": 35, "vibrance": 30, "saturation": -10,
        "curveShadows": 30, "curveHighlights": -25, "pointCurve": [[0, 10], [128, 140], [255, 245]],
        "hslHue": [10, 0, 0, 0, 0, -15, 0, 0], "hslSat": [0, 20, 0, -30, 0, 25, 0, 0],
        "gradeShadows": {"hue": 195, "sat": 28, "lum": 0}, "gradeHighlights": {"hue": 35, "sat": 22, "lum": 0}}"#;
    let ajustes = revela_core::ajustes::sanear(json).unwrap();
    let mut ed = Editor::new();
    ed.open("a.png", &src, 24.0, 2048).unwrap();
    let ours = photocraft_codecs::decode(&ed.export(ExportSettings { format: ExportFormat::Png, ..Default::default() }, &ajustes).unwrap()).unwrap();

    // El mismo revelado hecho por el comando del motor sobre la foto entera.
    let mut s = photocraft_engine::Session::new();
    s.add_document(photocraft_io::import("a.png", &src).unwrap().document, None);
    s.execute("filter.cameraRaw", serde_json::from_str(json).unwrap()).unwrap();
    let engine = photocraft_io::export(&s.active().unwrap().doc, "png", &Default::default()).unwrap().bytes;
    let engine = photocraft_codecs::decode(&engine).unwrap();
    let (a, b) = (ours.convert(ChannelLayout::Rgb, SampleType::U8), engine.convert(ChannelLayout::Rgb, SampleType::U8));
    let max = a.data().iter().zip(b.data()).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0);
    assert!(max <= 1, "diferencia máxima {max}/255");
}

#[test]
fn transparency_is_kept_in_png_and_matted_in_jpg() {
    let mut ed = Editor::new();
    ed.open("t.png", &png(64, 32, true), 24.0, 2048).unwrap();
    let p = photocraft_codecs::decode(&ed.export(ExportSettings { format: ExportFormat::Png, ..Default::default() }, &Default::default()).unwrap()).unwrap();
    assert!(p.layout().has_alpha());
    let j = photocraft_codecs::decode(&ed.export(ExportSettings::default(), &Default::default()).unwrap()).unwrap();
    let j = j.convert(ChannelLayout::Rgb, SampleType::U8);
    // La esquina transparente queda blanca en el JPG.
    assert!(j.data()[..3].iter().all(|v| *v > 245), "{:?}", &j.data()[..3]);
}

#[test]
fn garbage_is_an_error_not_a_crash() {
    let mut ed = Editor::new();
    assert!(ed.open("x.jpg", b"no es una foto", 24.0, 2048).is_err());
    assert!(ed.export(ExportSettings::default(), &Default::default()).is_err());
    assert!(ed.preview(100).is_err());
}
