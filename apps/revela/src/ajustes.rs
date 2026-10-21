//! Ajustes de foto: el subconjunto **por píxel** del filtro Camera Raw de PhotoCraft.
//!
//! La previa los dibuja en la GPU (shader WebGL2, `web/src/lib/gpu.ts`) y la exportación los
//! aplica con el motor (`camera_raw::develop`) por franjas. Para que las dos se vean igual:
//!
//! - [`sanear`] deja pasar solo las etapas que no miran a los vecinos: temperatura/tinte y
//!   exposición, contraste, vibrancia/saturación, curva paramétrica, curvas de puntos, HSL y
//!   gradación de color. Esas dan el mismo resultado enteras que por franjas (0/255, medido).
//! - [`Gpu::de`] calcula aquí, con la misma aritmética `f32` que `develop`, todas las constantes
//!   que el shader necesita, y las curvas las genera el propio motor. El shader no recalcula nada
//!   por su cuenta.
//! - Una prueba en el CI compara el shader contra el motor (≤ 1/255) y bloquea el despliegue.

use photocraft_algo::camera_raw::{CameraRaw, Wheel, curve_lut, develop, validate_curve};

use crate::efectos::{ConstPuntual, MAX_PUNTUALES, Puntual, Vineta};
use crate::{Error, Result};

/// Todo lo que se aplica al revelar: los ajustes por píxel del motor y la viñeta (que Revela
/// aplica con coordenadas del encuadre entero, ver `efectos`).
#[derive(Debug, Clone, Default)]
pub struct Revelado {
    pub raw: CameraRaw,
    /// Colores puntuales, en orden.
    pub puntuales: Vec<ConstPuntual>,
    pub vineta: Vineta,
}

/// Lo que Revela añade a la receta de Camera Raw (el motor lo ignora).
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Extras {
    puntuales: Vec<Puntual>,
}

/// La receta del usuario (JSON de Camera Raw) lista para revelar.
pub fn revelado(json: &str) -> Result<Revelado> {
    let raw = sanear(json)?;
    let p: CameraRaw = serde_json::from_str(json).map_err(|e| Error(format!("ajustes no válidos: {e}")))?;
    let extras: Extras = serde_json::from_str(json).map_err(|e| Error(format!("ajustes no válidos: {e}")))?;
    let puntuales = extras.puntuales.iter().take(MAX_PUNTUALES).map(Puntual::constantes).collect();
    Ok(Revelado { raw, puntuales, vineta: Vineta::de(&p) })
}

/// Entradas de cada curva que recibe el shader (las mismas que usa `develop`).
pub const LUT: usize = 1024;

/// La receta del usuario (JSON con los nombres de Camera Raw, `camelCase`) reducida a las etapas
/// por píxel y con cada valor dentro de su rango. Lo que no está en la lista se ignora.
pub fn sanear(json: &str) -> Result<CameraRaw> {
    let p: CameraRaw = serde_json::from_str(json).map_err(|e| Error(format!("ajustes no válidos: {e}")))?;
    for c in [&p.point_curve, &p.point_curve_red, &p.point_curve_green, &p.point_curve_blue] {
        if c.len() >= 2 {
            validate_curve(c).map_err(|e| Error(format!("curva no válida: {e}")))?;
        }
    }
    let r = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 0.0 };
    let wheel = |w: &Wheel| Wheel { hue: r(w.hue, 0.0, 360.0), sat: r(w.sat, 0.0, 100.0), lum: r(w.lum, -100.0, 100.0) };
    let curve = |c: &Vec<[f32; 2]>| if c.len() >= 2 { c.clone() } else { Vec::new() };
    Ok(CameraRaw {
        temperature: r(p.temperature, -100.0, 100.0),
        tint: r(p.tint, -100.0, 100.0),
        exposure: r(p.exposure, -5.0, 5.0),
        contrast: r(p.contrast, -100.0, 100.0),
        vibrance: r(p.vibrance, -100.0, 100.0),
        saturation: r(p.saturation, -100.0, 100.0),
        curve_highlights: r(p.curve_highlights, -100.0, 100.0),
        curve_lights: r(p.curve_lights, -100.0, 100.0),
        curve_darks: r(p.curve_darks, -100.0, 100.0),
        curve_shadows: r(p.curve_shadows, -100.0, 100.0),
        curve_splits: p.curve_splits.map(|v| r(v, 5.0, 95.0)),
        point_curve: curve(&p.point_curve),
        point_curve_red: curve(&p.point_curve_red),
        point_curve_green: curve(&p.point_curve_green),
        point_curve_blue: curve(&p.point_curve_blue),
        hsl_hue: p.hsl_hue.map(|v| r(v, -100.0, 100.0)),
        hsl_sat: p.hsl_sat.map(|v| r(v, -100.0, 100.0)),
        hsl_lum: p.hsl_lum.map(|v| r(v, -100.0, 100.0)),
        grade_shadows: wheel(&p.grade_shadows),
        grade_midtones: wheel(&p.grade_midtones),
        grade_highlights: wheel(&p.grade_highlights),
        grade_global: wheel(&p.grade_global),
        grade_blending: r(p.grade_blending, 0.0, 100.0),
        grade_balance: r(p.grade_balance, -100.0, 100.0),
        ..CameraRaw::default()
    })
}

/// Todo lo que el shader necesita, ya calculado. Cada campo replica un paso de `develop`.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpu {
    /// Etapa 1 (temperatura/tinte/exposición en luz lineal) activa, y sus ganancias por canal.
    pub wb: bool,
    pub gain: [f32; 3],
    /// Etapa 3: `contrast / 100 * 0.6` (0 = sin contraste).
    pub contrast_k: f32,
    /// Etapas 4–7 activas (alguna de vibrancia, saturación, curvas, HSL o gradación).
    pub color: bool,
    pub saturation: f32,
    pub vibrance: f32,
    /// Curvas activas: paramétrica, maestra, roja, verde, azul (en ese orden en `luts`).
    pub lut_on: [bool; 5],
    pub luts: Vec<f32>,
    pub hsl: bool,
    pub hsl_hue: [f32; 8],
    pub hsl_sat: [f32; 8],
    pub hsl_lum: [f32; 8],
    pub grade: bool,
    pub tints: [[f32; 3]; 4],
    pub lums: [f32; 4],
    pub blend: f32,
    pub balance: f32,
    /// La viñeta (sus constantes; la posición la pone la interfaz según el encuadre).
    pub vineta: Option<crate::efectos::Constantes>,
    /// Los colores puntuales (como mucho `MAX_PUNTUALES`).
    pub puntuales: Vec<ConstPuntual>,
}

fn luma(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn is_linear(c: &[[f32; 2]]) -> bool {
    c.len() < 2 || c.iter().all(|p| (p[0] - p[1]).abs() < 1e-3)
}

impl Gpu {
    pub fn de_revelado(r: &Revelado) -> Gpu {
        Gpu { vineta: r.vineta.activa().then(|| r.vineta.constantes()), puntuales: r.puntuales.clone(), ..Gpu::de(&r.raw) }
    }

    pub fn de(p: &CameraRaw) -> Gpu {
        let wb = !(p.temperature == 0.0 && p.tint == 0.0 && p.exposure == 0.0);
        let mut gain = photocraft_algo::camera_raw::white_balance_gains(p.temperature, p.tint);
        let norm = luma(gain);
        gain = gain.map(|v| v / norm * 2f32.powf(p.exposure));

        let par_on = !(p.curve_highlights == 0.0 && p.curve_lights == 0.0 && p.curve_darks == 0.0 && p.curve_shadows == 0.0);
        let curves = [&p.point_curve, &p.point_curve_red, &p.point_curve_green, &p.point_curve_blue];
        let lut_on = [par_on, !is_linear(curves[0]), !is_linear(curves[1]), !is_linear(curves[2]), !is_linear(curves[3])];
        let identity: Vec<f32> = (0..LUT).map(|i| i as f32 / (LUT - 1) as f32).collect();
        let mut luts = Vec::with_capacity(5 * LUT);
        luts.extend(if par_on { parametric_lut(p) } else { identity.clone() });
        for (on, c) in lut_on.iter().skip(1).zip(curves) {
            luts.extend(if *on { curve_lut(c, LUT) } else { identity.clone() });
        }

        let hsl = !p.hsl_hue.iter().chain(&p.hsl_sat).chain(&p.hsl_lum).all(|v| *v == 0.0);
        let neutral = |w: &Wheel| w.sat == 0.0 && w.lum == 0.0;
        let wheels = [&p.grade_shadows, &p.grade_midtones, &p.grade_highlights, &p.grade_global];
        let grade = !wheels.iter().all(|w| neutral(w));
        let tints = wheels.map(|w| {
            let c = hsv_to_rgb(w.hue, 1.0, 1.0);
            let l = luma(c);
            [c[0] - l, c[1] - l, c[2] - l].map(|v| v * w.sat / 100.0 * 0.3)
        });
        let color = p.vibrance != 0.0 || p.saturation != 0.0 || lut_on.iter().any(|b| *b) || hsl || grade;
        Gpu {
            wb,
            gain,
            contrast_k: p.contrast / 100.0 * 0.6,
            color,
            saturation: p.saturation / 100.0,
            vibrance: p.vibrance / 100.0,
            lut_on,
            luts,
            hsl,
            hsl_hue: p.hsl_hue,
            hsl_sat: p.hsl_sat,
            hsl_lum: p.hsl_lum,
            grade,
            tints,
            lums: wheels.map(|w| w.lum),
            blend: 0.15 + p.grade_blending.clamp(0.0, 100.0) / 100.0 * 0.35,
            balance: p.grade_balance.clamp(-100.0, 100.0) / 100.0 * 0.25,
            vineta: None,
            puntuales: Vec::new(),
        }
    }
}

/// La curva paramétrica tal como la calcula el motor: una rampa de grises pasada por `develop`
/// con solo la curva paramétrica activa devuelve exactamente su tabla (la función del motor es
/// privada; así no hay que copiarla).
fn parametric_lut(p: &CameraRaw) -> Vec<f32> {
    let only = CameraRaw {
        curve_highlights: p.curve_highlights,
        curve_lights: p.curve_lights,
        curve_darks: p.curve_darks,
        curve_shadows: p.curve_shadows,
        curve_splits: p.curve_splits,
        ..CameraRaw::default()
    };
    let mut ramp: Vec<[f32; 4]> = (0..LUT).map(|i| {
        let v = i as f32 / (LUT - 1) as f32;
        [v, v, v, 1.0]
    }).collect();
    develop(&mut ramp, LUT, 1, &only, false);
    ramp.iter().map(|q| q[0]).collect()
}

/// Igual que el `hsv_to_rgb` del motor (privado), para los tintes de la gradación.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_controls_never_reach_the_export() {
        let p = sanear(r#"{"shadows": 60, "highlights": -60, "clarity": 40, "dehaze": 20, "whites": 10, "vignetteAmount": -30, "grainAmount": 20, "sharpenAmount": 50, "exposure": 0.5}"#).unwrap_or_default();
        assert_eq!(p.exposure, 0.5);
        assert_eq!((p.shadows, p.highlights, p.clarity, p.dehaze, p.whites), (0.0, 0.0, 0.0, 0.0, 0.0));
        assert_eq!((p.vignette_amount, p.grain_amount, p.sharpen_amount), (0.0, 0.0, 0.0));
    }

    #[test]
    fn values_are_clamped_and_bad_input_is_an_error() {
        let p = sanear(r#"{"exposure": 99, "contrast": -500, "temperature": 1e30}"#).unwrap_or_default();
        assert_eq!((p.exposure, p.contrast, p.temperature), (5.0, -100.0, 100.0));
        assert!(sanear("no es json").is_err());
        assert!(sanear(r#"{"pointCurve": [[0, 0], [0, 10], [255, 255]]}"#).is_err());
    }

    #[test]
    fn the_parametric_lut_is_the_engines() {
        let p = sanear(r#"{"curveShadows": 40, "curveHighlights": -30}"#).unwrap_or_default();
        let g = Gpu::de(&p);
        assert!(g.lut_on[0] && g.color);
        let lut = &g.luts[..LUT];
        assert!(lut.windows(2).all(|w| w[1] >= w[0]), "monótona");
        assert!(lut[LUT / 8] > LUT as f32 / 8.0 / (LUT - 1) as f32, "las sombras suben");
        // Un gris pasado por el motor cae sobre la tabla.
        let mut px = [[0.25f32, 0.25, 0.25, 1.0]];
        develop(&mut px, 1, 1, &p, false);
        let x = 0.25 * (LUT - 1) as f32;
        let i = x as usize;
        let expected = lut[i] + (lut[i + 1] - lut[i]) * (x - i as f32);
        assert!((px[0][0] - expected).abs() < 1e-6);
    }

    #[test]
    fn neutral_settings_switch_every_stage_off() {
        let g = Gpu::de(&sanear("{}").unwrap_or_default());
        assert!(!g.wb && g.contrast_k == 0.0 && !g.color && !g.hsl && !g.grade);
    }
}
