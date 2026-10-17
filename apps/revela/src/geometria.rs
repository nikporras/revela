//! Geometría: girar 90°, voltear, enderezar y recortar.
//!
//! Todo se reduce a **una transformación afín** que lleva cada píxel de la salida a un punto de la
//! foto de origen ([`Plano`]), y a un muestreo **bilineal** idéntico en la GPU (previa, `gpu.ts`) y
//! aquí (exportación y prueba de igualdad). Por eso la previa y la exportación coinciden, igual
//! que con los ajustes de color.
//!
//! Orden (lo que ve el usuario): la foto se gira `giro` × 90° en el sentido del reloj, luego se
//! voltea (`espejo_h`, `espejo_v`), luego se endereza `angulo` grados (positivo: en el sentido
//! del reloj, alrededor del centro) y por último se recorta el rectángulo `recorte`, expresado en
//! fracciones del marco ya girado y volteado (`[cx, cy, ancho, alto]`, centro y tamaño).

use serde::Deserialize;

use crate::{Error, Result};

/// El ángulo de enderezado se limita a ±45° (más allá, es un giro de 90° más un enderezado).
pub const ANGULO_MAX: f64 = 45.0;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Geometria {
    /// Giros de 90° en el sentido del reloj (0..=3).
    pub giro: u8,
    pub espejo_h: bool,
    pub espejo_v: bool,
    /// Grados; positivo = en el sentido del reloj.
    pub angulo: f64,
    /// `[cx, cy, ancho, alto]` en fracciones del marco girado (0..1). `None`: todo el marco.
    pub recorte: Option<[f64; 4]>,
}

impl Default for Geometria {
    fn default() -> Self {
        Self { giro: 0, espejo_h: false, espejo_v: false, angulo: 0.0, recorte: None }
    }
}

/// Lo que hay que dibujar: el tamaño de la salida y la transformación (en `f32`, la misma que
/// recibe la GPU) que lleva el centro de cada píxel de la salida, `(u + ½, v + ½)`, a coordenadas
/// de la foto de origen: `x = m[0]·u' + m[1]·v' + m[2]`, `y = m[3]·u' + m[4]·v' + m[5]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plano {
    pub ancho: u32,
    pub alto: u32,
    pub m: [f32; 6],
    /// Tamaño de la foto de origen.
    pub origen: (u32, u32),
}

impl Geometria {
    /// La geometría de un JSON de la interfaz, saneada. `""` o `"{}"`: ninguna.
    pub fn de_json(json: &str) -> Result<Geometria> {
        if json.trim().is_empty() {
            return Ok(Geometria::default());
        }
        let g: Geometria = serde_json::from_str(json).map_err(|e| Error(format!("geometría no válida: {e}")))?;
        let angulo = if g.angulo.is_finite() { g.angulo.clamp(-ANGULO_MAX, ANGULO_MAX) } else { 0.0 };
        let recorte = g.recorte.filter(|r| r.iter().all(|v| v.is_finite())).map(|[cx, cy, w, h]| {
            let w = w.clamp(1e-4, 1.0);
            let h = h.clamp(1e-4, 1.0);
            [cx.clamp(w / 2.0, 1.0 - w / 2.0), cy.clamp(h / 2.0, 1.0 - h / 2.0), w, h]
        });
        Ok(Geometria { giro: g.giro % 4, angulo, recorte, ..g })
    }

    pub fn es_identidad(&self) -> bool {
        self.giro == 0 && !self.espejo_h && !self.espejo_v && self.angulo == 0.0 && self.recorte.is_none_or(|r| r == [0.5, 0.5, 1.0, 1.0])
    }

    /// Tamaño del marco girado para una foto de `w`×`h`.
    pub fn marco(&self, w: u32, h: u32) -> (u32, u32) {
        if self.giro % 2 == 1 { (h, w) } else { (w, h) }
    }

    /// El plano para una foto de origen de `w`×`h` (la de trabajo al exportar, la copia de previa
    /// en pantalla: el mismo recorte en fracciones da el mismo encuadre a cualquier tamaño).
    pub fn plano(&self, w: u32, h: u32) -> Plano {
        let (fw, fh) = self.marco(w, h);
        let (fw, fh) = (f64::from(fw), f64::from(fh));
        let [cx, cy, cw, ch] = self.recorte.unwrap_or([0.5, 0.5, 1.0, 1.0]);
        let ancho = ((cw * fw).round() as u32).max(1);
        let alto = ((ch * fh).round() as u32).max(1);
        // Salida (u', v') → marco enderezado P: esquina del recorte + paso por píxel.
        let (x0, y0) = ((cx - cw / 2.0) * fw, (cy - ch / 2.0) * fh);
        let (sx, sy) = (cw * fw / f64::from(ancho), ch * fh / f64::from(alto));
        // P → marco girado Q: deshacer el enderezado (girar −ángulo alrededor del centro).
        let (sn, cs) = (-self.angulo).to_radians().sin_cos();
        let (mx, my) = (fw / 2.0, fh / 2.0);
        // Q = [cs −sn; sn cs]·(P − C) + C, con P = (x0 + sx·u', y0 + sy·v').
        let q = [cs * sx, -sn * sy, cs * (x0 - mx) - sn * (y0 - my) + mx, sn * sx, cs * sy, sn * (x0 - mx) + cs * (y0 - my) + my];
        // Q → origen S: deshacer los volteos y luego el giro.
        let (w, h) = (f64::from(w), f64::from(h));
        let mut a = q;
        if self.espejo_h {
            a = [-a[0], -a[1], fw - a[2], a[3], a[4], a[5]];
        }
        if self.espejo_v {
            a = [a[0], a[1], a[2], -a[3], -a[4], fh - a[5]];
        }
        // Fila x y fila y de Q, para componer el giro inverso.
        let (qx, qy) = ([a[0], a[1], a[2]], [a[3], a[4], a[5]]);
        let neg = |r: [f64; 3], c: f64| [-r[0], -r[1], c - r[2]];
        let (ox, oy) = match self.giro % 4 {
            // 90° en el sentido del reloj: (x, y) → (H − y, x); al revés: x = Qy, y = H − Qx.
            1 => (qy, neg(qx, h)),
            2 => (neg(qx, w), neg(qy, h)),
            // 270°: (x, y) → (y, W − x); al revés: x = W − Qy, y = Qx.
            3 => (neg(qy, w), qx),
            _ => (qx, qy),
        };
        let m = [ox[0], ox[1], ox[2], oy[0], oy[1], oy[2]].map(|v| v as f32);
        Plano { ancho, alto, m, origen: (w as u32, h as u32) }
    }
}

impl Plano {
    /// Una ventana de la salida (`[x0, y0, x1, y1]` en fracciones, 0..1) dibujada a
    /// `ancho`×`alto` píxeles: el zoom de la previa. Con la ventana entera y el tamaño de la
    /// salida, es el mismo plano.
    pub fn ventana(&self, [x0, y0, x1, y1]: [f64; 4], ancho: u32, alto: u32) -> Plano {
        let (ancho, alto) = (ancho.max(1), alto.max(1));
        let (w, h) = (f64::from(self.ancho), f64::from(self.alto));
        // Píxel de la ventana a' → salida u' = ox + sx·a'.
        let (ox, oy) = (x0 * w, y0 * h);
        let (sx, sy) = ((x1 - x0) * w / f64::from(ancho), (y1 - y0) * h / f64::from(alto));
        let m = self.m.map(f64::from);
        let m = [m[0] * sx, m[1] * sy, m[0] * ox + m[1] * oy + m[2], m[3] * sx, m[4] * sy, m[3] * ox + m[4] * oy + m[5]].map(|v| v as f32);
        Plano { ancho, alto, m, origen: self.origen }
    }

    /// Punto de la foto de origen para el centro del píxel `(u, v)` de la salida.
    pub fn origen_de(&self, u: u32, v: u32) -> (f32, f32) {
        let (pu, pv) = (u as f32 + 0.5, v as f32 + 0.5);
        let m = &self.m;
        (m[0] * pu + m[1] * pv + m[2], m[3] * pu + m[4] * pv + m[5])
    }

    /// Rectángulo (en píxeles de origen, ya recortado a la foto) que hace falta leer para la zona
    /// `[u0, u1) × [v0, v1)` de la salida: sus esquinas transformadas, más un píxel de margen para
    /// la interpolación. `None` si la zona cae fuera de la foto.
    pub fn huella(&self, u0: u32, v0: u32, u1: u32, v1: u32) -> Option<[i32; 4]> {
        let m = &self.m;
        let esq = [(u0, v0), (u1, v0), (u0, v1), (u1, v1)].map(|(u, v)| {
            let (u, v) = (u as f32, v as f32);
            (m[0] * u + m[1] * v + m[2], m[3] * u + m[4] * v + m[5])
        });
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (x, y) in esq {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        let (w, h) = (self.origen.0 as i32, self.origen.1 as i32);
        let r = [((x0 - 1.5).floor() as i32).max(0), ((y0 - 1.5).floor() as i32).max(0), ((x1 + 1.5).ceil() as i32).min(w), ((y1 + 1.5).ceil() as i32).min(h)];
        (r[0] < r[2] && r[1] < r[3]).then_some(r)
    }
}

/// Muestreo bilineal en el punto `(x, y)` de la foto de origen (`w`×`h`, centros de píxel en
/// `i + ½`), con los vecinos sujetos al borde. Fuera de la foto: transparente. Es exactamente lo
/// que hace el shader (`muestra` en `gpu.ts`), en `f32`.
pub fn bilineal(x: f32, y: f32, w: u32, h: u32, texel: impl Fn(i32, i32) -> [f32; 4]) -> [f32; 4] {
    if !(x >= 0.0 && y >= 0.0 && x <= w as f32 && y <= h as f32) {
        return [0.0; 4];
    }
    let (fx, fy) = (x - 0.5, y - 0.5);
    let (ix, iy) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - ix, fy - iy);
    let (wi, hi) = (w as i32 - 1, h as i32 - 1);
    let (x0, y0) = ((ix as i32).clamp(0, wi), (iy as i32).clamp(0, hi));
    let (x1, y1) = ((ix as i32 + 1).clamp(0, wi), (iy as i32 + 1).clamp(0, hi));
    let (a, b, c, d) = (texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1));
    let mut out = [0.0f32; 4];
    for k in 0..4 {
        let arriba = a[k] + (b[k] - a[k]) * tx;
        let abajo = c[k] + (d[k] - c[k]) * tx;
        out[k] = arriba + (abajo - arriba) * ty;
    }
    out
}

/// Ángulo (grados, en el sentido del reloj) que deja horizontal o vertical la línea `a → b`
/// dibujada sobre la foto: la regla de "Enderezar capa" del motor.
pub fn angulo_de_linea(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = photocraft_doc::Ruler { start: [a.0, a.1], end: [b.0, b.1], protractor: None };
    photocraft_engine::analysis_cmds::straighten_angle(&r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(json: &str) -> Geometria {
        Geometria::de_json(json).unwrap_or_default()
    }

    /// Dónde cae en el origen el centro del píxel (u, v).
    fn donde(p: &Plano, u: u32, v: u32) -> (f32, f32) {
        let (x, y) = p.origen_de(u, v);
        ((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0)
    }

    #[test]
    fn identity_maps_each_pixel_to_itself() {
        let p = g("{}").plano(40, 30);
        assert_eq!((p.ancho, p.alto), (40, 30));
        assert_eq!(donde(&p, 0, 0), (0.5, 0.5));
        assert_eq!(donde(&p, 39, 29), (39.5, 29.5));
        assert!(g("{}").es_identidad() && g(r#"{"recorte": [0.5, 0.5, 1, 1]}"#).es_identidad());
    }

    #[test]
    fn quarter_turns_and_flips_land_on_pixel_centres() {
        // 90° en el sentido del reloj: la esquina superior izquierda de la salida es la inferior
        // izquierda del origen.
        let p = g(r#"{"giro": 1}"#).plano(40, 30);
        assert_eq!((p.ancho, p.alto), (30, 40));
        assert_eq!(donde(&p, 0, 0), (0.5, 29.5));
        assert_eq!(donde(&p, 29, 0), (0.5, 0.5));
        let p = g(r#"{"giro": 3}"#).plano(40, 30);
        assert_eq!(donde(&p, 0, 0), (39.5, 0.5));
        let p = g(r#"{"giro": 2}"#).plano(40, 30);
        assert_eq!(donde(&p, 0, 0), (39.5, 29.5));
        let p = g(r#"{"espejoH": true}"#).plano(40, 30);
        assert_eq!(donde(&p, 0, 0), (39.5, 0.5));
        let p = g(r#"{"espejoV": true}"#).plano(40, 30);
        assert_eq!(donde(&p, 0, 0), (0.5, 29.5));
    }

    #[test]
    fn straightening_turns_about_the_centre() {
        let p = g(r#"{"angulo": 90}"#).plano(40, 40); // se limita a 45
        let (x, y) = p.origen_de(20, 20);
        assert!((x - 20.5).abs() < 0.8 && (y - 20.5).abs() < 0.8, "el centro se queda: {x},{y}");
        // +10°: un punto a la derecha del centro en la salida viene de un poco más arriba
        // (la foto se giró en el sentido del reloj).
        let p = g(r#"{"angulo": 10}"#).plano(100, 100);
        let (_, y) = p.origen_de(90, 50);
        assert!(y < 50.0, "{y}");
    }

    #[test]
    fn the_crop_selects_a_window_of_the_turned_frame() {
        let p = g(r#"{"recorte": [0.25, 0.5, 0.5, 1.0]}"#).plano(40, 30);
        assert_eq!((p.ancho, p.alto), (20, 30));
        assert_eq!(donde(&p, 0, 0), (0.5, 0.5));
        // El recorte se expresa en el marco girado.
        let p = g(r#"{"giro": 1, "recorte": [0.5, 0.25, 1.0, 0.5]}"#).plano(40, 30);
        assert_eq!((p.ancho, p.alto), (30, 20));
        assert_eq!(donde(&p, 0, 0), (0.5, 29.5));
    }

    #[test]
    fn a_window_zooms_into_the_output() {
        let p = g(r#"{"angulo": 7, "giro": 1}"#).plano(400, 300);
        // La ventana entera al mismo tamaño es el mismo plano.
        let v = p.ventana([0.0, 0.0, 1.0, 1.0], p.ancho, p.alto);
        assert_eq!((v.ancho, v.alto), (p.ancho, p.alto));
        assert!(v.m.iter().zip(p.m).all(|(a, b)| (a - b).abs() < 1e-3), "{:?} {:?}", v.m, p.m);
        // El cuarto inferior derecho a la misma escala: el píxel (0, 0) de la ventana es el
        // (ancho/2, alto/2) de la salida.
        let v = p.ventana([0.5, 0.5, 1.0, 1.0], p.ancho / 2, p.alto / 2);
        let (a, b) = (v.origen_de(0, 0), p.origen_de(p.ancho / 2, p.alto / 2));
        assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3, "{a:?} {b:?}");
        // A mitad de tamaño, cada píxel de la ventana cubre dos de la salida.
        let v = p.ventana([0.0, 0.0, 1.0, 1.0], p.ancho / 2, p.alto / 2);
        let (a, b, c) = (v.origen_de(0, 0), p.origen_de(0, 0), p.origen_de(1, 1));
        assert!((a.0 - (b.0 + c.0) / 2.0).abs() < 1e-3 && (a.1 - (b.1 + c.1) / 2.0).abs() < 1e-3);
    }

    #[test]
    fn bilinear_matches_texels_at_centres_and_blends_between() {
        let t = |x: i32, y: i32| [x as f32, y as f32, 0.0, 1.0];
        assert_eq!(bilineal(2.5, 1.5, 4, 3, t), [2.0, 1.0, 0.0, 1.0]);
        assert_eq!(bilineal(3.0, 1.5, 4, 3, t), [2.5, 1.0, 0.0, 1.0]);
        assert_eq!(bilineal(-0.1, 1.0, 4, 3, t), [0.0; 4]);
        // En el borde, el vecino se sujeta.
        assert_eq!(bilineal(4.0, 3.0, 4, 3, t), [3.0, 2.0, 0.0, 1.0]);
    }

    #[test]
    fn footprint_covers_the_tile_and_stays_inside_the_photo() {
        let p = g(r#"{"angulo": 20}"#).plano(400, 300);
        let r = p.huella(0, 0, 64, 64).unwrap_or([0; 4]);
        assert!(r[0] >= 0 && r[1] >= 0 && r[2] <= 400 && r[3] <= 300);
        for (u, v) in [(0, 0), (63, 0), (0, 63), (63, 63), (30, 30)] {
            let (x, y) = p.origen_de(u, v);
            if x >= 0.0 && y >= 0.0 && x <= 400.0 && y <= 300.0 {
                assert!(x >= r[0] as f32 && x <= r[2] as f32 && y >= r[1] as f32 && y <= r[3] as f32);
            }
        }
    }

    #[test]
    fn the_line_rule_is_the_engines() {
        // Una línea que sube 10° hacia la derecha se endereza girando 10° en el sentido del reloj.
        let t = 10f64.to_radians();
        let a = angulo_de_linea((0.0, 100.0), (100.0 * t.cos(), 100.0 - 100.0 * t.sin()));
        assert!((a - 10.0).abs() < 1e-9, "{a}");
        // Casi vertical: se endereza a vertical.
        let a = angulo_de_linea((50.0, 0.0), (55.0, 100.0));
        assert!(a.abs() < 5.0, "{a}");
    }

    #[test]
    fn bad_input_is_sanitised() {
        let p = g(r#"{"giro": 7, "angulo": 1e9, "recorte": [2, -1, 5, 0]}"#);
        assert_eq!(p.giro, 3);
        assert_eq!(p.angulo, ANGULO_MAX);
        let r = p.recorte.unwrap_or_default();
        assert!(r[2] <= 1.0 && r[3] > 0.0 && r[0] - r[2] / 2.0 >= 0.0 && r[1] - r[3] / 2.0 >= -1e-12);
        assert!(Geometria::de_json("no").is_err());
    }
}
