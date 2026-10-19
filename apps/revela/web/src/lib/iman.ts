// Imán sutil para sliders (`use:iman`): al cruzar el valor neutro (de un lado al otro) el
// slider se detiene en él, con una vibración corta (como la rueda de gradación). No atrae desde
// cerca, así que valores pequeños (1, 2…) se pueden poner desde los dos lados. Un doble toque
// lo devuelve al neutro.
//
// Escucha `input` en el propio <input>: Svelte 5 delega `oninput` en la raíz, así que este
// oyente corre antes y el componente ya recibe el valor pegado.

export interface OpcionesIman {
  /** Valor neutro (0 por defecto). */
  neutro?: number;
  /** Distancia a la que también se pega al acercarse; por defecto 0 (solo al cruzarlo). */
  zona?: number;
}

export function iman(el: HTMLInputElement, opciones: OpcionesIman = {}) {
  let o = opciones;
  const neutro = () => o.neutro ?? 0;
  const zona = () => o.zona ?? 0;
  let anterior = Number(el.value);

  function alMover() {
    const n = neutro();
    const v = Number(el.value);
    // Al pasar al otro lado del neutro se detiene en él (y, si hay zona, al acercarse mucho).
    // Alejándose no se pega.
    const cruza = Math.sign(v - n) !== Math.sign(anterior - n);
    const acerca = Math.abs(v - n) < Math.abs(anterior - n) && Math.abs(v - n) <= zona();
    const llega = anterior !== n && v !== n && (cruza || acerca);
    if (llega) el.value = String(n);
    const ahora = Number(el.value);
    if (ahora === n && anterior !== n) navigator.vibrate?.(8);
    anterior = ahora;
  }

  // Doble toque: dos toques seguidos cerca uno del otro.
  let ultimo = { t: -1e9, x: 0 };
  function alTocar(e: PointerEvent) {
    // El mismo <input> puede pasar a otro ajuste (los botones de Luz y Color): se parte del valor
    // que tiene ahora.
    anterior = Number(el.value);
    if (e.timeStamp - ultimo.t < 300 && Math.abs(e.clientX - ultimo.x) < 40) {
      ultimo = { t: -1e9, x: 0 };
      // Tras el toque el navegador mueve el control al dedo; después se devuelve al neutro.
      requestAnimationFrame(() => {
        const n = neutro();
        if (Number(el.value) !== n || anterior !== n) navigator.vibrate?.(8);
        el.value = String(n);
        anterior = n;
        el.dispatchEvent(new Event('input', { bubbles: true }));
        el.dispatchEvent(new Event('change', { bubbles: true }));
      });
      return;
    }
    ultimo = { t: e.timeStamp, x: e.clientX };
  }

  el.addEventListener('input', alMover);
  el.addEventListener('pointerdown', alTocar);
  return {
    update(nuevas: OpcionesIman = {}) {
      o = nuevas;
      anterior = Number(el.value);
    },
    destroy() {
      el.removeEventListener('input', alMover);
      el.removeEventListener('pointerdown', alTocar);
    },
  };
}
