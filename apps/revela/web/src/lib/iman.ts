// Imán para sliders (`use:iman`): cerca del valor neutro se pega a él, con una vibración corta
// al llegar (como la rueda de gradación), y un doble toque lo devuelve al neutro.
//
// Escucha `input` en el propio <input>: Svelte 5 delega `oninput` en la raíz, así que este
// oyente corre antes y el componente ya recibe el valor pegado.

export interface OpcionesIman {
  /** Valor neutro (0 por defecto). */
  neutro?: number;
  /** Distancia a la que se pega; por defecto, el 1,5 % del recorrido. */
  zona?: number;
}

export function iman(el: HTMLInputElement, opciones: OpcionesIman = {}) {
  let o = opciones;
  const neutro = () => o.neutro ?? 0;
  const zona = () => o.zona ?? (Number(el.max) - Number(el.min)) * 0.015;
  let anterior = Number(el.value);

  function alMover() {
    const n = neutro();
    const v = Number(el.value);
    if (v !== n && Math.abs(v - n) <= zona()) el.value = String(n);
    const ahora = Number(el.value);
    if (ahora === n && anterior !== n) navigator.vibrate?.(8);
    anterior = ahora;
  }

  // Doble toque: dos toques seguidos cerca uno del otro.
  let ultimo = { t: -1e9, x: 0 };
  function alTocar(e: PointerEvent) {
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
