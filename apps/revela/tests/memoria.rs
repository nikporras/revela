//! Pico de memoria por etapa (abrir, previa, exportar) según el pico de memoria residente (Linux).
//!
//!     REVELA_FOTO=foto.jpg cargo test -p revela-core --release --test memoria -- --ignored --nocapture

/// `VmHWM` (pico de memoria residente) o `VmRSS` de /proc/self/status, en bytes.
fn status(campo: &str) -> usize {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines().find_map(|l| l.strip_prefix(campo)?.trim().strip_suffix(" kB")?.trim().parse::<usize>().ok()).unwrap_or(0) * 1024
}

fn etapa<T>(nombre: &str, f: impl FnOnce() -> T) -> T {
    // Escribir 5 en clear_refs reinicia el pico (VmHWM) al consumo actual.
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    let antes = status("VmRSS:");
    let t = std::time::Instant::now();
    let r = f();
    let mb = |b: usize| b as f64 / 1048576.0;
    println!(
        "{nombre:<10} pico {:>7.1} MB (+{:>6.1} sobre lo que había) · queda {:>7.1} MB · {:?}",
        mb(status("VmHWM:")),
        mb(status("VmHWM:").saturating_sub(antes)),
        mb(status("VmRSS:")),
        t.elapsed()
    );
    r
}

#[test]
#[ignore = "necesita REVELA_FOTO; mide, no comprueba"]
fn pico_por_etapa() {
    let ruta = std::env::var("REVELA_FOTO").expect("REVELA_FOTO=ruta de una foto");
    let bytes = std::fs::read(&ruta).unwrap();
    let mut ed = revela_core::Editor::new();
    let info = etapa("abrir", || ed.open(&ruta, &bytes, 24.0, 2048).unwrap());
    println!("           {}×{} ({:.1} MP)", info.width, info.height, f64::from(info.width) * f64::from(info.height) / 1e6);
    etapa("previa", || ed.preview(2048).unwrap());
    etapa("exportar", || ed.export(Default::default()).unwrap());
}
