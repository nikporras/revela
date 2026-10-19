// Comportamiento de los sliders (`use:iman` en el <input type="range">):
//
// - Arrastre relativo con ajuste fino: el valor se mueve lo que se mueve el dedo, sin saltar al
//   punto tocado; cuanto más se aleja el dedo hacia arriba del slider, más despacio (½, ¼, 1/10),
//   con un rótulo junto al dedo. El arrastre lo lleva el contenedor del <input> (que no recibe
//   el dedo); el <input> sigue sirviendo para el teclado y la accesibilidad.
// - Imán sutil: al cruzar el valor neutro se detiene en él (con una vibración corta) y hay que
//   seguir unos píxeles para salir. No atrae desde cerca: 1 o 2 se pueden poner.
// - Doble toque: vuelve al neutro.
//
// Los cambios salen como eventos `input` y, al soltar, `change`, igual que un slider normal.

export interface OpcionesIman {
  /** Valor neutro (0 por defecto). */
  neutro?: number;
}

/** Velocidad según lo alejado que esté el dedo por encima del slider (px). */
function velocidad(arriba: number): number {
  if (arriba < 40) return 1;
  if (arriba < 100) return 1 / 2;
  if (arriba < 170) return 1 / 4;
  return 1 / 10;
}

const NOMBRE_VELOCIDAD = new Map([
  [1 / 2, 'Ajuste fino ½'],
  [1 / 4, 'Ajuste fino ¼'],
  [1 / 10, 'Ajuste fino 1/10'],
]);

/** Píxeles que hay que seguir arrastrando para salir del neutro tras cruzarlo. */
const RETENCION = 12;

export function iman(el: HTMLInputElement, opciones: OpcionesIman = {}) {
  let o = opciones;
  const neutro = () => o.neutro ?? 0;
  const min = () => Number(el.min);
  const max = () => Number(el.max);
  const paso = () => Number(el.step) || 1;
  const caja = el.parentElement ?? el;
  // El dedo lo lleva el contenedor; el <input> no recibe el puntero (así no salta ni arrastra
  // por su cuenta).
  el.style.pointerEvents = 'none';
  caja.style.touchAction = 'none';

  function emitir(v: number, tipo: 'input' | 'change' = 'input') {
    el.value = String(v);
    el.dispatchEvent(new Event(tipo, { bubbles: true }));
  }

  // --- Teclado: el imán al cruzar el neutro -------------------------------------------------
  let anterior = Number(el.value);
  function alTeclado() {
    if (arrastre) return;
    const n = neutro();
    const v = Number(el.value);
    if (anterior !== n && v !== n && Math.sign(v - n) !== Math.sign(anterior - n)) {
      el.value = String(n);
      navigator.vibrate?.(8);
    }
    anterior = Number(el.value);
  }

  // --- Arrastre ---------------------------------------------------------------------------
  let arrastre: {
    id: number;
    x: number;
    /** Borde de arriba del slider: el ajuste fino se mide desde ahí. */
    y0: number;
    valor: number;
    /** El valor sin redondear al paso (para que el ajuste fino acumule). */
    exacto: number;
    retencion: number;
    movido: boolean;
    rotulo: HTMLDivElement | null;
  } | null = null;
  let ultimoToque = { t: -1e9, x: 0 };

  function rotular(e: PointerEvent, texto: string | undefined) {
    if (!arrastre) return;
    if (!texto) {
      arrastre.rotulo?.remove();
      arrastre.rotulo = null;
      return;
    }
    if (!arrastre.rotulo) {
      const r = document.createElement('div');
      r.setAttribute('role', 'status');
      r.className = 'rotulo-fino';
      Object.assign(r.style, {
        position: 'fixed',
        zIndex: '50',
        padding: '4px 10px',
        borderRadius: '999px',
        background: '#000c',
        color: '#fff',
        font: '600 13px system-ui, sans-serif',
        pointerEvents: 'none',
        transform: 'translate(-50%, -100%)',
        whiteSpace: 'nowrap',
      });
      document.body.append(r);
      arrastre.rotulo = r;
    }
    arrastre.rotulo.textContent = texto;
    arrastre.rotulo.style.left = `${e.clientX}px`;
    arrastre.rotulo.style.top = `${e.clientY - 56}px`;
  }

  function bajar(e: PointerEvent) {
    if (el.disabled || !e.isPrimary) return;
    e.preventDefault();
    // Doble toque: dos toques seguidos cerca uno del otro.
    if (e.timeStamp - ultimoToque.t < 300 && Math.abs(e.clientX - ultimoToque.x) < 40) {
      ultimoToque = { t: -1e9, x: 0 };
      const n = neutro();
      if (Number(el.value) !== n) navigator.vibrate?.(8);
      emitir(n);
      emitir(n, 'change');
      anterior = n;
      return;
    }
    ultimoToque = { t: e.timeStamp, x: e.clientX };
    try {
      caja.setPointerCapture(e.pointerId);
    } catch {
      // Un puntero sintético o ya inactivo: se sigue sin captura.
    }
    const v = Number(el.value);
    arrastre = { id: e.pointerId, x: e.clientX, y0: caja.getBoundingClientRect().top, valor: v, exacto: v, retencion: 0, movido: false, rotulo: null };
  }

  function mover(e: PointerEvent) {
    const a = arrastre;
    if (!a || e.pointerId !== a.id) return;
    let dx = e.clientX - a.x;
    a.x = e.clientX;
    if (dx === 0) return;
    a.movido = true;
    const k = velocidad(Math.max(0, a.y0 - e.clientY));
    rotular(e, NOMBRE_VELOCIDAD.get(k));
    // Tras cruzar el neutro, unos píxeles quieto en él.
    if (a.retencion > 0) {
      const usar = Math.min(a.retencion, Math.abs(dx));
      a.retencion -= usar;
      dx -= Math.sign(dx) * usar;
      if (dx === 0) return;
    }
    const ancho = el.getBoundingClientRect().width || 1;
    const n = neutro();
    const antes = a.exacto;
    let v = Math.min(max(), Math.max(min(), antes + (dx / ancho) * (max() - min()) * k));
    if (antes !== n && v !== n && Math.sign(v - n) !== Math.sign(antes - n)) {
      v = n;
      a.retencion = RETENCION;
      navigator.vibrate?.(8);
    }
    a.exacto = v;
    // Al paso del slider, sin ruido de coma flotante.
    const p = paso();
    const decimales = (String(p).split('.')[1] ?? '').length;
    const q = Number((Math.round((v - min()) / p) * p + min()).toFixed(decimales));
    if (q !== Number(el.value)) emitir(q);
  }

  function soltar(e: PointerEvent) {
    const a = arrastre;
    if (!a || e.pointerId !== a.id) return;
    a.rotulo?.remove();
    arrastre = null;
    anterior = Number(el.value);
    if (a.movido && Number(el.value) !== a.valor) el.dispatchEvent(new Event('change', { bubbles: true }));
  }

  el.addEventListener('input', alTeclado);
  caja.addEventListener('pointerdown', bajar);
  caja.addEventListener('pointermove', mover);
  caja.addEventListener('pointerup', soltar);
  caja.addEventListener('pointercancel', soltar);
  return {
    update(nuevas: OpcionesIman = {}) {
      o = nuevas;
      anterior = Number(el.value);
    },
    destroy() {
      arrastre?.rotulo?.remove();
      el.removeEventListener('input', alTeclado);
      caja.removeEventListener('pointerdown', bajar);
      caja.removeEventListener('pointermove', mover);
      caja.removeEventListener('pointerup', soltar);
      caja.removeEventListener('pointercancel', soltar);
    },
  };
}
