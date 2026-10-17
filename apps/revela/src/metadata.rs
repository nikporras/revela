//! Metadatos al exportar: privacidad por defecto.
//!
//! El motor copia el bloque EXIF original casi intacto. Revela lo limpia antes de exportar:
//!
//! - **Ubicación GPS** (etiqueta 0x8825 en IFD0): se borra, salvo que el usuario pida conservarla.
//! - **Miniatura EXIF** (IFD1): se borra siempre. Muestra la foto *sin editar*, así que dejaría
//!   ver lo que se recortó.
//!
//! Se borra de verdad: los bytes de esas estructuras se ponen a cero, no solo se desenlazan.
//! Si el bloque EXIF no se puede analizar con seguridad (BigTIFF, datos rotos), se descarta
//! entero: mejor perder la fecha de la cámara que filtrar la ubicación.

use photocraft_doc::Metadata;
use std::sync::Arc;

const TAG_GPS_IFD: u16 = 0x8825;
const ENTRY: usize = 12;
const PREFIX: &[u8] = b"Exif\0\0";

/// Los metadatos que se escriben en una exportación.
pub fn for_export(meta: &Metadata, keep_location: bool) -> Metadata {
    let exif = meta.exif.as_deref().and_then(|e| clean_exif(e, keep_location)).map(Arc::new);
    // El texto libre (PNG tEXt…) y el XMP pueden repetir datos privados: no se exportan.
    Metadata { xmp: None, exif, text: Vec::new(), psd_resources: Vec::new(), psd_global_blocks: Vec::new() }
}

/// El bloque EXIF sin miniatura y, salvo `keep_location`, sin GPS. `None` si hay que descartarlo.
pub fn clean_exif(exif: &[u8], keep_location: bool) -> Option<Vec<u8>> {
    let (prefix, body) = match exif.strip_prefix(PREFIX) {
        Some(body) => (PREFIX, body),
        None => (&[][..], exif),
    };
    let mut b = body.to_vec();
    let t = Tiff::parse(&b)?;
    let ifd0 = t.u32(&b, 4)? as usize;
    // IFD1 (la miniatura) primero: su puntero está al final de IFD0, que luego puede moverse.
    let next_at = t.next_pointer_at(&b, ifd0)?;
    let ifd1 = t.u32(&b, next_at)? as usize;
    if ifd1 != 0 {
        t.wipe_ifd(&mut b, ifd1, true)?;
        t.put_u32(&mut b, next_at, 0)?;
    }
    if !keep_location {
        while let Some(i) = t.find(&b, ifd0, TAG_GPS_IFD)? {
            let e = ifd0.checked_add(2)?.checked_add(i.checked_mul(ENTRY)?)?;
            let gps = t.u32(&b, e.checked_add(8)?)? as usize;
            t.wipe_ifd(&mut b, gps, false)?;
            t.remove_entry(&mut b, ifd0, i)?;
        }
    }
    let mut out = prefix.to_vec();
    out.extend_from_slice(&b);
    Some(out)
}

#[derive(Clone, Copy)]
struct Tiff {
    little: bool,
}

impl Tiff {
    /// Solo TIFF clásico (offsets de 32 bits); BigTIFF devuelve `None`.
    fn parse(b: &[u8]) -> Option<Self> {
        match b.get(0..4)? {
            [b'I', b'I', 42, 0] => Some(Self { little: true }),
            [b'M', b'M', 0, 42] => Some(Self { little: false }),
            _ => None,
        }
    }

    fn u16(self, b: &[u8], at: usize) -> Option<u16> {
        let s: [u8; 2] = b.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(if self.little { u16::from_le_bytes(s) } else { u16::from_be_bytes(s) })
    }

    fn u32(self, b: &[u8], at: usize) -> Option<u32> {
        let s: [u8; 4] = b.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(if self.little { u32::from_le_bytes(s) } else { u32::from_be_bytes(s) })
    }

    fn put_u16(self, b: &mut [u8], at: usize, v: u16) -> Option<()> {
        let s = if self.little { v.to_le_bytes() } else { v.to_be_bytes() };
        b.get_mut(at..at.checked_add(2)?)?.copy_from_slice(&s);
        Some(())
    }

    fn put_u32(self, b: &mut [u8], at: usize, v: u32) -> Option<()> {
        let s = if self.little { v.to_le_bytes() } else { v.to_be_bytes() };
        b.get_mut(at..at.checked_add(4)?)?.copy_from_slice(&s);
        Some(())
    }

    /// Número de entradas de la IFD en `ifd`, comprobando que la tabla completa cabe.
    fn count(self, b: &[u8], ifd: usize) -> Option<usize> {
        if ifd < 8 {
            return None;
        }
        let n = usize::from(self.u16(b, ifd)?);
        let end = ifd.checked_add(2)?.checked_add(n.checked_mul(ENTRY)?)?.checked_add(4)?;
        (end <= b.len()).then_some(n)
    }

    fn next_pointer_at(self, b: &[u8], ifd: usize) -> Option<usize> {
        let n = self.count(b, ifd)?;
        ifd.checked_add(2)?.checked_add(n.checked_mul(ENTRY)?)
    }

    /// Índice de la primera entrada con `tag`. `Some(None)` si no está; `None` si la IFD está rota.
    fn find(self, b: &[u8], ifd: usize, tag: u16) -> Option<Option<usize>> {
        let n = self.count(b, ifd)?;
        for i in 0..n {
            let e = ifd.checked_add(2)?.checked_add(i.checked_mul(ENTRY)?)?;
            if self.u16(b, e)? == tag {
                return Some(Some(i));
            }
        }
        Some(None)
    }

    /// Pone a cero los valores fuera de línea de cada entrada y la propia tabla de la IFD. Con
    /// `thumbnail`, también los bytes JPEG de la miniatura (etiquetas 0x0201/0x0202).
    fn wipe_ifd(self, b: &mut [u8], ifd: usize, thumbnail: bool) -> Option<()> {
        let n = self.count(b, ifd)?;
        let mut jpeg = (None, None);
        for i in 0..n {
            let e = ifd.checked_add(2)?.checked_add(i.checked_mul(ENTRY)?)?;
            let tag = self.u16(b, e)?;
            let size = type_size(self.u16(b, e.checked_add(2)?)?).checked_mul(self.u32(b, e.checked_add(4)?)? as usize)?;
            let value = self.u32(b, e.checked_add(8)?)? as usize;
            if size > 4 {
                zero(b, value, size);
            }
            match tag {
                0x0201 => jpeg.0 = Some(value),
                0x0202 => jpeg.1 = Some(value),
                _ => {}
            }
        }
        if let (true, (Some(at), Some(len))) = (thumbnail, jpeg) {
            zero(b, at, len);
        }
        zero(b, ifd, 2usize.checked_add(n.checked_mul(ENTRY)?)?.checked_add(4)?);
        Some(())
    }

    /// Quita la entrada `i`: sube las siguientes y el puntero a la próxima IFD, y pone a cero los
    /// 12 bytes que quedan libres al final. Ningún otro offset cambia.
    fn remove_entry(self, b: &mut [u8], ifd: usize, i: usize) -> Option<()> {
        let n = self.count(b, ifd)?;
        let first = ifd.checked_add(2)?;
        let at = first.checked_add(i.checked_mul(ENTRY)?)?;
        let end = first.checked_add(n.checked_mul(ENTRY)?)?.checked_add(4)?;
        b.get_mut(at..end)?.copy_within(ENTRY.., 0);
        zero(b, end.checked_sub(ENTRY)?, ENTRY);
        self.put_u16(b, ifd, u16::try_from(n.checked_sub(1)?).ok()?)
    }
}

/// Bytes por valor de cada tipo TIFF (los desconocidos cuentan como 1).
fn type_size(ty: u16) -> usize {
    match ty {
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 => 8,
        _ => 1,
    }
}

/// Pone a cero `len` bytes desde `at`, recortado al final del bloque.
fn zero(b: &mut [u8], at: usize, len: usize) {
    let end = at.saturating_add(len).min(b.len());
    if let Some(s) = b.get_mut(at.min(end)..end) {
        s.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EXIF little-endian con IFD0 = {Orientation, GPS → IFD GPS}, IFD GPS = {latitud (3
    /// racionales, fuera de línea)} e IFD1 = {miniatura JPEG de 4 bytes}.
    fn sample() -> Vec<u8> {
        let mut b = b"II*\0".to_vec();
        b.extend(8u32.to_le_bytes()); // IFD0 en 8
        // IFD0: 2 entradas (8..38)
        b.extend(2u16.to_le_bytes());
        b.extend([0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0]); // Orientation = 6
        b.extend([0x25, 0x88, 4, 0, 1, 0, 0, 0, 38, 0, 0, 0]); // GPS IFD en 38
        b.extend(80u32.to_le_bytes()); // IFD1 en 80
        // IFD GPS (38..56): 1 entrada; latitud en 56 (24 bytes)
        b.extend(1u16.to_le_bytes());
        b.extend([0x02, 0x00, 5, 0, 3, 0, 0, 0, 56, 0, 0, 0]);
        b.extend(0u32.to_le_bytes());
        b.extend([0xAA; 24]); // 56..80
        // IFD1 (80..110): miniatura en 110, 4 bytes
        b.extend(2u16.to_le_bytes());
        b.extend([0x01, 0x02, 4, 0, 1, 0, 0, 0, 110, 0, 0, 0]);
        b.extend([0x02, 0x02, 4, 0, 1, 0, 0, 0, 4, 0, 0, 0]);
        b.extend(0u32.to_le_bytes());
        b.extend([0xFF, 0xD8, 0xFF, 0xD9]);
        b
    }

    #[test]
    fn removes_gps_and_thumbnail_by_default() {
        let out = clean_exif(&sample(), false).unwrap_or_default();
        assert_eq!(out.len(), sample().len());
        let t = Tiff { little: true };
        assert_eq!(t.find(&out, 8, TAG_GPS_IFD), Some(None));
        assert_eq!(t.find(&out, 8, 0x0112), Some(Some(0)));
        assert_eq!(t.u32(&out, t.next_pointer_at(&out, 8).unwrap_or(0)), Some(0));
        assert!(!out.windows(4).any(|w| w == [0xAA; 4]), "la latitud quedó en el archivo");
        assert!(!out.windows(2).any(|w| w == [0xFF, 0xD8]), "la miniatura quedó en el archivo");
    }

    #[test]
    fn keeps_gps_when_asked_but_never_the_thumbnail() {
        let out = clean_exif(&sample(), true).unwrap_or_default();
        let t = Tiff { little: true };
        assert_eq!(t.find(&out, 8, TAG_GPS_IFD), Some(Some(1)));
        assert!(out.windows(4).any(|w| w == [0xAA; 4]));
        assert!(!out.windows(2).any(|w| w == [0xFF, 0xD8]));
    }

    #[test]
    fn keeps_the_jpeg_prefix() {
        let mut e = PREFIX.to_vec();
        e.extend(sample());
        assert!(clean_exif(&e, false).unwrap_or_default().starts_with(PREFIX));
    }

    #[test]
    fn broken_exif_is_dropped_not_kept() {
        assert_eq!(clean_exif(b"II*\0\xff\xff\xff\x7f", false), None);
        assert_eq!(clean_exif(b"basura", false), None);
        let mut s = sample();
        s.truncate(30);
        assert_eq!(clean_exif(&s, false), None);
        let meta = Metadata { exif: Some(Arc::new(b"basura".to_vec())), xmp: Some("<x/>".into()), ..Default::default() };
        let out = for_export(&meta, false);
        assert!(out.exif.is_none() && out.xmp.is_none());
    }
}
