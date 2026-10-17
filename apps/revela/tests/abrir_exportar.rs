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

    let out = ed.export(ExportSettings::default()).unwrap();
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
    let out = ed.export(ExportSettings { keep_location: true, ..Default::default() }).unwrap();
    assert!(has_gps_bytes(&out));
}

#[test]
fn png_export_has_no_location_either() {
    let mut ed = Editor::new();
    ed.open("a.jpg", &camera_jpeg(), 24.0, 2048).unwrap();
    let out = ed.export(ExportSettings { format: ExportFormat::Png, ..Default::default() }).unwrap();
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
    let out = photocraft_codecs::decode(&ed.export(ExportSettings::default()).unwrap()).unwrap();
    assert_eq!((out.width(), out.height()), (40, 60));
}

#[test]
fn garbage_is_an_error_not_a_crash() {
    let mut ed = Editor::new();
    assert!(ed.open("x.jpg", b"no es una foto", 24.0, 2048).is_err());
    assert!(ed.export(ExportSettings::default()).is_err());
    assert!(ed.preview(100).is_err());
}
