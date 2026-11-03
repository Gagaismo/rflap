// The step state machine every replay uses (conversion steps, transformations, parser traces) and the
// control bar that drives it: previous / next / do all, keyboard and automatic play.

import { h } from './dom.js';
import { t } from './i18n.js';

export class Replay {
  constructor(count = 0) {
    this.count = Math.max(0, count);
    this.index = 0;
  }

  /** 1-based position shown to the user; 0 when there is nothing to replay. */
  get position() {
    return this.count === 0 ? 0 : this.index + 1;
  }

  get atStart() {
    return this.index <= 0;
  }

  get atEnd() {
    return this.count === 0 || this.index >= this.count - 1;
  }

  /** Every mover answers whether the index changed. */
  goTo(to) {
    if (!Number.isFinite(to) || this.count === 0) return false;
    const next = Math.max(0, Math.min(this.count - 1, Math.trunc(to)));
    const moved = next !== this.index;
    this.index = next;
    return moved;
  }

  next() {
    return this.goTo(this.index + 1);
  }

  prev() {
    return this.goTo(this.index - 1);
  }

  first() {
    return this.goTo(0);
  }

  last() {
    return this.goTo(this.count - 1);
  }

  /** JFLAP's "do all": jump to the end. */
  doAll() {
    return this.last();
  }

  reset(count, { keep = false } = {}) {
    this.count = Math.max(0, count);
    this.index = keep ? Math.max(0, Math.min(this.count - 1, this.index)) : 0;
  }
}

const KEYS = {
  ArrowRight: 'next',
  n: 'next',
  ArrowLeft: 'prev',
  p: 'prev',
  Home: 'first',
  End: 'last',
  a: 'last',
  ' ': 'play',
};

/** The replay action a key press asks for, or null (application shortcuts are never taken). */
export function replayKeyAction(event) {
  if (event.ctrlKey || event.metaKey || event.altKey) return null;
  return KEYS[event.key] || null;
}

/**
 * The control bar. `onChange()` redraws after any move. Returns `{node, stop}`; `stop` ends the automatic
 * play (call it from the panel's destroy). Keys work while the bar's container has focus.
 */
export function replayBar(replay, onChange, { interval = 900, key = '' } = {}) {
  let timer = null;
  const stop = () => {
    if (timer) clearInterval(timer);
    timer = null;
  };
  let node = null;
  /** The panels redraw everything on a change: keep the keyboard focus where it was (same button, or the region). */
  const refocus = () => {
    const active = document.activeElement;
    const index = [...node.querySelectorAll('button')].indexOf(active);
    const inside = node.parentElement && node.parentElement.contains(active);
    return () => {
      if (!key || (index < 0 && !inside)) return;
      const fresh = document.querySelector(`[data-replay-key="${key}"]`);
      if (!fresh) return;
      const target = index >= 0 ? [...fresh.querySelectorAll('button')][index] : null;
      if (target && !target.disabled) target.focus();
      else if (fresh.parentElement) fresh.parentElement.focus();
    };
  };
  const act = (name) => {
    const restore = node ? refocus() : () => {};
    if (name === 'play') {
      if (timer) stop();
      else if (!replay.atEnd) timer = setInterval(() => (replay.next() ? onChange() : (stop(), onChange())), interval);
    } else {
      stop();
      replay[name === 'last' ? 'doAll' : name]();
    }
    onChange();
    restore();
  };
  const button = (label, name, disabled, title) => h('button', { type: 'button', title: title || label, 'aria-label': title || label, disabled, onclick: () => act(name) }, label);
  node = h(
    'div',
    { class: 'button-row replay-bar', role: 'group', 'aria-label': t('replay.controls'), 'data-replay-key': key || null },
    button('⏮', 'first', replay.atStart, t('replay.first')),
    button(`◀ ${t('replay.previous')}`, 'prev', replay.atStart),
    h('span', { role: 'status', 'aria-live': 'polite' }, t('replay.position', { n: replay.position, total: replay.count })),
    button(`${t('replay.next')} ▶`, 'next', replay.atEnd),
    button(`${t('replay.do_all')} ⏭`, 'last', replay.atEnd),
    button(timer ? `⏸ ${t('replay.pause')}` : `⏵ ${t('replay.play')}`, 'play', replay.atEnd && !timer),
  );
  return { node, stop, act };
}

/** Wires the keyboard to a container: n/→ next, p/← previous, Home, End/a (do all), Space (play). */
export function replayKeys(container, act) {
  container.setAttribute('tabindex', '0');
  container.addEventListener('keydown', (event) => {
    const target = event.target;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')) return;
    const action = replayKeyAction(event);
    if (action) {
      event.preventDefault();
      act(action);
    }
  });
}
