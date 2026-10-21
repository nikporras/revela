//! Efectos que dependen de la posición en el encuadre final: la viñeta.
//!
//! El motor (`camera_raw::develop`) la calcula sobre el trozo que recibe, así que al exportar por
//! franjas o bloques saldría una viñeta en cada uno. Aquí está **su misma fórmula**
//! (`crates/algo/src/camera_raw.rs`, `fn vignette`), en `f32` y con la misma aritmética, pero
//! con coordenadas del encuadre entero. Una prueba la compara con el motor sobre la foto entera.
//!
//! Va al final del revelado y antes de recortar a 0..1, como en el motor: `develop` se llama con
//! `float = true` y la viñeta se aplica después.

use photocraft_algo::camera_raw::CameraRaw;

/// La viñeta de una receta, saneada. `cantidad == 0`: sin viñeta.
#[derive(Debug, Clone, PartialEq)]
pub struct Vineta {
    pub cantidad: f32,
    pub punto_medio: f32,
    pub redondez: f32,
    pub difuminado: f32,
    pub luces: f32,
    pub estilo: Estilo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estilo {
    /// El del motor por defecto ("highlightPriority"): multiplica los tres canales.
    PrioridadLuces,
    /// "colorPriority": cambia la luminancia y conserva el color.
    PrioridadColor,
    /// "paintOverlay": funde hacia negro o blanco.
    Pintura,
}

impl Estilo {
    pub fn indice(self) -> i32 {
        match self {
            Estilo::PrioridadLuces => 0,
            Estilo::PrioridadColor => 1,
            Estilo::Pintura => 2,
        }
    }
}

impl Default for Vineta {
    fn default() -> Self {
        Vineta::de(&CameraRaw::default())
    }
}

impl Vineta {
    /// La viñeta de unos ajustes de Camera Raw (los de la receta del usuario).
    pub fn de(p: &CameraRaw) -> Vineta {
        let r = |v: f32, lo: f32, hi: f32, def: f32| if v.is_finite() { v.clamp(lo, hi) } else { def };
        Vineta {
            cantidad: r(p.vignette_amount, -100.0, 100.0, 0.0),
            punto_medio: r(p.vignette_midpoint, 0.0, 100.0, 50.0),
            redondez: r(p.vignette_roundness, -100.0, 100.0, 0.0),
            difuminado: r(p.vignette_feather, 0.0, 100.0, 50.0),
            luces: r(p.vignette_highlights, 0.0, 100.0, 0.0),
            estilo: match p.vignette_style.as_str() {
                "paintOverlay" => Estilo::Pintura,
                "colorPriority" => Estilo::PrioridadColor,
                _ => Estilo::PrioridadLuces,
            },
        }
    }

    pub fn activa(&self) -> bool {
        self.cantidad != 0.0
    }

    /// Las constantes que no dependen del tamaño del encuadre (las que recibe el shader).
    pub fn constantes(&self) -> Constantes {
        let mid = self.punto_medio / 100.0;
        let feather = self.difuminado / 100.0;
        let round = self.redondez / 100.0;
        let inner = 1.0 - mid * 0.8;
        Constantes {
            amt: self.cantidad / 100.0,
            round,
            pw: 2.0 + (-round).max(0.0) * 6.0,
            edge0: inner - feather * 0.5 * inner,
            edge1: inner + feather * 0.5 * (1.4 - inner) + 0.02,
            hl: self.luces / 100.0,
            estilo: self.estilo,
        }
    }

    /// La viñeta para un encuadre de `w`×`h` píxeles.
    pub fn plano(&self, w: u32, h: u32) -> PlanoVineta {
        let k = self.constantes();
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let (ax, ay) = ejes(k.round, cx, cy);
        PlanoVineta { k, cx, cy, ax, ay }
    }
}

/// Lo que no depende del tamaño del encuadre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constantes {
    pub amt: f32,
    pub round: f32,
    pub pw: f32,
    pub edge0: f32,
    pub edge1: f32,
    pub hl: f32,
    pub estilo: Estilo,
}

/// Semiejes de la viñeta: 0 sigue la proporción del encuadre, +1 un círculo, −1 un rectángulo
/// redondeado (con `pw`).
pub fn ejes(round: f32, cx: f32, cy: f32) -> (f32, f32) {
    if round >= 0.0 {
        let r = cx.min(cy) + (cx.max(cy) - cx.min(cy)) * (1.0 - round);
        (if cx >= cy { r } else { cx }, if cy > cx { r } else { cy })
    } else {
        (cx, cy)
    }
}

/// La viñeta para un encuadre concreto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanoVineta {
    pub k: Constantes,
    pub cx: f32,
    pub cy: f32,
    pub ax: f32,
    pub ay: f32,
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

impl PlanoVineta {
    /// Aplica la viñeta a un bloque de `tw` píxeles de ancho cuya esquina está en `(x0, y0)` del
    /// encuadre.
    pub fn aplicar(&self, px: &mut [[f32; 4]], tw: usize, x0: usize, y0: usize) {
        let k = &self.k;
        for (i, q) in px.iter_mut().enumerate() {
            let (x, y) = ((x0 + i % tw) as f32, (y0 + i / tw) as f32);
            let (dx, dy) = (((x + 0.5 - self.cx) / self.ax).abs(), ((y + 0.5 - self.cy) / self.ay).abs());
            let r = (dx.powf(k.pw) + dy.powf(k.pw)).powf(1.0 / k.pw);
            let m = smoothstep(k.edge0, k.edge1, r);
            if m <= 0.0 {
                continue;
            }
            let l = luma([q[0], q[1], q[2]]).clamp(0.0, 1.0);
            match k.estilo {
                Estilo::Pintura => {
                    let target = if k.amt < 0.0 { 0.0 } else { 1.0 };
                    let f = m * k.amt.abs();
                    for v in &mut q[..3] {
                        *v += (target - *v) * f;
                    }
                }
                _ => {
                    let mut f = 2f32.powf(k.amt * 2.0 * m);
                    if k.amt < 0.0 && k.hl > 0.0 {
                        // Las luces atraviesan una viñeta que oscurece.
                        let protect = smoothstep(0.6, 1.0, l) * k.hl;
                        f += (1.0 - f) * protect;
                    }
                    if k.estilo == Estilo::PrioridadColor {
                        let nl = l * f;
                        for v in &mut q[..3] {
                            *v += nl - l;
                        }
                    } else {
                        for v in &mut q[..3] {
                            *v *= f;
                        }
                    }
                }
            }
        }
    }
}

// --- Color puntual -------------------------------------------------------------------------
//
// Como el Point Color de Lightroom: se elige un color de la foto y solo los píxeles parecidos
// cambian de tono, saturación y luminancia. No existe en el motor: es una etapa de Revela, que
// va después del revelado del motor (tras el HSL y la gradación) y antes de la viñeta. La GPU
// hace exactamente estas cuentas (`gpu.ts`); la prueba de igualdad las compara.

/** Como mucho, tantos colores puntuales. */
pub const MAX_PUNTUALES: usize = 8;

/// Un color puntual de la receta (JSON `puntuales`, en `camelCase`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Puntual {
    /// El color elegido, en HSV: tono 0..360, saturación y brillo 0..1.
    pub tono0: f32,
    pub sat0: f32,
    pub brillo0: f32,
    /// 0..100: lo ancho del rango de colores parecidos (50 por defecto).
    pub rango: f32,
    /// −100..100.
    pub tono: f32,
    pub saturacion: f32,
    pub luminancia: f32,
}

impl Default for Puntual {
    fn default() -> Self {
        Puntual { tono0: 0.0, sat0: 0.0, brillo0: 0.0, rango: 50.0, tono: 0.0, saturacion: 0.0, luminancia: 0.0 }
    }
}

/// Lo que necesita la cuenta de cada píxel (lo mismo que recibe el shader).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstPuntual {
    pub h0: f32,
    pub s0: f32,
    pub v0: f32,
    /// Anchos de la caída en tono (grados), saturación y brillo.
    pub ancho_h: f32,
    pub ancho_s: f32,
    pub ancho_v: f32,
    /// Giro de tono (grados), y cambios de saturación y luminancia (fracciones).
    pub dh: f32,
    pub ds: f32,
    pub dl: f32,
}

impl Puntual {
    /// Saneado (valores en rango). Se conservan también los que aún no cambian nada: la vista de
    /// zonas afectadas los señala por su posición en la lista.
    pub fn constantes(&self) -> ConstPuntual {
        let r = |v: f32, lo: f32, hi: f32, def: f32| if v.is_finite() { v.clamp(lo, hi) } else { def };
        let k = r(self.rango, 0.0, 100.0, 50.0) / 100.0;
        let (dh, ds, dl) = (r(self.tono, -100.0, 100.0, 0.0), r(self.saturacion, -100.0, 100.0, 0.0), r(self.luminancia, -100.0, 100.0, 0.0));
        ConstPuntual {
            h0: r(self.tono0, 0.0, 360.0, 0.0),
            s0: r(self.sat0, 0.0, 1.0, 0.0),
            v0: r(self.brillo0, 0.0, 1.0, 0.0),
            ancho_h: 8.0 + 52.0 * k,
            ancho_s: 0.15 + 0.6 * k,
            ancho_v: 0.2 + 0.6 * k,
            dh: dh / 100.0 * 45.0,
            ds: ds / 100.0,
            dl: dl / 100.0,
        }
    }
}

/// `rgb_to_hsv` del motor (`camera_raw.rs`; es privada), igual que `rgb2hsv` del shader.
pub fn rgb_a_hsv(c: [f32; 3]) -> [f32; 3] {
    let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    let d = mx - mn;
    let h = if d <= 1e-9 {
        0.0
    } else if mx == c[0] {
        60.0 * (((c[1] - c[2]) / d).rem_euclid(6.0))
    } else if mx == c[1] {
        60.0 * ((c[2] - c[0]) / d + 2.0)
    } else {
        60.0 * ((c[0] - c[1]) / d + 4.0)
    };
    [h, if mx > 1e-9 { d / mx } else { 0.0 }, mx]
}

/// `hsv_to_rgb` del motor, igual que `hsv2rgb` del shader.
pub fn hsv_a_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r + m, g + m, b + m]
}

impl ConstPuntual {
    /// Cuánto se parece un color (en HSV) al elegido: 1 igual, 0 nada.
    pub fn peso(&self, hsv: [f32; 3]) -> f32 {
        let [h, s, v] = hsv;
        let dh = ((h - self.h0 + 540.0).rem_euclid(360.0) - 180.0).abs();
        let fh = 1.0 - smoothstep(self.ancho_h * 0.5, self.ancho_h, dh);
        let fs = 1.0 - smoothstep(self.ancho_s * 0.5, self.ancho_s, (s - self.s0).abs());
        let fv = 1.0 - smoothstep(self.ancho_v * 0.5, self.ancho_v, (v - self.v0).abs());
        // En los grises el tono no significa nada: no se tocan.
        fh * fs * fv * smoothstep(0.02, 0.1, s)
    }
}

/// Aplica los colores puntuales, uno tras otro, a cada píxel.
pub fn aplicar_puntuales(px: &mut [[f32; 4]], puntos: &[ConstPuntual]) {
    if puntos.is_empty() {
        return;
    }
    for q in px.iter_mut() {
        let mut c = [q[0], q[1], q[2]];
        for p in puntos {
            let [h, s, v] = rgb_a_hsv(c.map(|x| x.max(0.0)));
            if s <= 1e-4 {
                continue;
            }
            let w = p.peso([h, s, v]);
            if w <= 0.0 {
                continue;
            }
            let ns = (s * (1.0 + w * p.ds)).clamp(0.0, 1.0);
            let mut o = hsv_a_rgb(h + w * p.dh, ns, v);
            // Se conserva la luminancia y luego se aplica su cambio (como el HSL del motor).
            let (l0, l1) = (luma(c), luma(o));
            if l1 > 1e-6 {
                o = o.map(|x| x * l0 / l1);
            }
            let k = (w * p.dl).exp2();
            c = o.map(|x| x * k);
        }
        q[0] = c[0];
        q[1] = c[1];
        q[2] = c[2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receta(json: &str) -> CameraRaw {
        serde_json::from_str(json).unwrap_or_default()
    }

    /// Una imagen de prueba con color en todas partes (para que se note cualquier diferencia).
    fn imagen(w: usize, h: usize) -> Vec<[f32; 4]> {
        (0..w * h).map(|i| [0.2 + 0.7 * (i % w) as f32 / w as f32, 0.4, 0.9 - 0.6 * (i / w) as f32 / h as f32, 1.0]).collect()
    }

    #[test]
    fn it_is_the_engines_vignette_with_global_coordinates() {
        let (w, h) = (90, 60);
        for json in [
            r#"{"vignetteAmount": -60}"#,
            r#"{"vignetteAmount": 45, "vignetteMidpoint": 20, "vignetteFeather": 90, "vignetteRoundness": 70}"#,
            r#"{"vignetteAmount": -80, "vignetteRoundness": -60, "vignetteHighlights": 70, "exposure": 1.2}"#,
            r#"{"vignetteAmount": -50, "vignetteStyle": "colorPriority"}"#,
            r#"{"vignetteAmount": 40, "vignetteStyle": "paintOverlay"}"#,
        ] {
            let p = receta(json);
            // El motor, sobre la imagen entera.
            let mut motor = imagen(w, h);
            photocraft_algo::camera_raw::develop(&mut motor, w, h, &p, false);
            // Nosotros: el revelado sin viñeta y la viñeta por bloques de 32×32.
            let sin = CameraRaw { vignette_amount: 0.0, ..p.clone() };
            let mut nuestro = imagen(w, h);
            photocraft_algo::camera_raw::develop(&mut nuestro, w, h, &sin, true);
            let v = Vineta::de(&p).plano(w as u32, h as u32);
            for by in (0..h).step_by(32) {
                for bx in (0..w).step_by(32) {
                    let (tw, th) = (32.min(w - bx), 32.min(h - by));
                    let mut bloque: Vec<[f32; 4]> = (0..th).flat_map(|j| nuestro[(by + j) * w + bx..(by + j) * w + bx + tw].to_vec()).collect();
                    v.aplicar(&mut bloque, tw, bx, by);
                    for j in 0..th {
                        nuestro[(by + j) * w + bx..(by + j) * w + bx + tw].copy_from_slice(&bloque[j * tw..(j + 1) * tw]);
                    }
                }
            }
            let max = motor
                .iter()
                .zip(&nuestro)
                .flat_map(|(a, b)| (0..3).map(move |c| (a[c] - b[c].clamp(0.0, 1.0)).abs()))
                .fold(0.0f32, f32::max);
            assert!(max < 1e-5, "{json}: diferencia {max}");
        }
    }

    #[test]
    fn a_point_colour_changes_only_similar_colours() {
        let p = Puntual { tono0: 30.0, sat0: 0.6, brillo0: 0.7, rango: 40.0, saturacion: -100.0, ..Default::default() };
        let k = [p.constantes()];
        // Un naranja como el elegido, uno que tira a amarillo (a mitad de la caída), un azul y
        // un gris.
        let mut px = vec![[0.7, 0.49, 0.28, 1.0], [0.7, 0.65, 0.25, 1.0], [0.2, 0.3, 0.7, 1.0], [0.5, 0.5, 0.5, 1.0]];
        let antes = px.clone();
        aplicar_puntuales(&mut px, &k);
        let sat = |q: &[f32; 4]| rgb_a_hsv([q[0], q[1], q[2]])[1];
        assert!(sat(&px[0]) < 0.05, "el elegido queda sin color: {:?}", px[0]);
        assert!(sat(&px[1]) < sat(&antes[1]) && sat(&px[1]) > 0.05, "el parecido cambia algo: {:?}", px[1]);
        assert_eq!(px[2], antes[2], "el azul no cambia");
        assert_eq!(px[3], antes[3], "el gris no cambia");
        // Sin ajustes no cambia nada (salvo el redondeo de ida y vuelta a HSV).
        let mut q = antes.clone();
        aplicar_puntuales(&mut q, &[Puntual { tono0: 30.0, sat0: 0.6, brillo0: 0.7, ..Default::default() }.constantes()]);
        let max = q.iter().zip(&antes).flat_map(|(a, b)| (0..3).map(move |c| (a[c] - b[c]).abs())).fold(0.0f32, f32::max);
        assert!(max < 1e-5, "{max}");
    }

    #[test]
    fn no_amount_no_vignette_and_values_are_sanitised() {
        assert!(!Vineta::de(&receta("{}")).activa());
        let v = Vineta::de(&receta(r#"{"vignetteAmount": 500, "vignetteMidpoint": -3}"#));
        assert_eq!((v.cantidad, v.punto_medio), (100.0, 0.0));
    }
}
