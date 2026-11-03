// Step viewer for the `steps` list a conversion returns (subset construction, minimization, state
// elimination, ...). Each step is shown readably and the states it mentions are highlighted: ids of the
// automaton that was converted on the canvas of its tab, ids of the result on this tab's canvas.

import { h, clear } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { stateList } from './doc.js';

/** Keys of a step whose numbers are state ids (directly, in arrays, or in objects with these keys). */
const ID_KEYS = new Set(['state', 'states', 'from', 'to', 'group', 'groups', 'into', 'pairs', 'edges', 'closure', 'members', 'sources', 'nfa_states', 'new_states', 'added', 'transition']);
/** Keys that always name states of the automaton the conversion started from. */
const SOURCE_KEYS = new Set(['nfa_states', 'group', 'groups', 'into', 'pairs', 'edges', 'closure', 'members', 'sources']);
/** Steps whose state / from / to are states of the RESULT (subset construction, minimization merge). */
const RESULT_KINDS = new Set(['new_state', 'add_transition', 'merge_group']);

export const kindOf = (step) => step.kind || step.action || '?';

/** 'source' (the automaton converted) or 'result' (the new one): the space the ids under `key` live in. */
export function spaceOf(step, key) {
  if (step.action && !step.kind) return 'result'; // regex -> NFA builds the result in place
  if (SOURCE_KEYS.has(key)) return 'source';
  if (key === 'new_states') return 'result';
  return RESULT_KINDS.has(kindOf(step)) ? 'result' : 'source';
}

/** Rebuilds the step replacing every state id by `fn(id, space)`. */
export function mapIds(step, fn) {
  const walk = (value, space) => {
    if (typeof value === 'number') return space ? fn(value, space) : value;
    if (Array.isArray(value)) return value.map((v) => walk(v, space));
    if (value && typeof value === 'object') {
      return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, ID_KEYS.has(k) ? walk(v, space || spaceOf(step, k)) : walk(v, null)]));
    }
    return value;
  };
  return Object.fromEntries(Object.entries(step).map(([k, v]) => [k, ID_KEYS.has(k) ? walk(v, spaceOf(step, k)) : walk(v, null)]));
}

/** Every state id a step mentions: { source: [...], result: [...] }. */
export function collectStateIds(step) {
  const found = { source: new Set(), result: new Set() };
  mapIds(step, (id, space) => {
    found[space].add(id);
    return id;
  });
  return { source: [...found.source], result: [...found.result] };
}

/** The step with state ids replaced by names, for display. */
export function resolveStep(step, nameOfSource, nameOfResult) {
  return mapIds(step, (id, space) => (space === 'source' ? nameOfSource(id) : nameOfResult(id)));
}

const show = (value) => {
  if (value === null || value === undefined) return '—';
  if (typeof value === 'string') return value === '' ? 'λ' : value;
  if (Array.isArray(value) && value.every((v) => typeof v !== 'object' || v === null)) return `{${value.map(show).join(', ')}}`;
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
};

const pretty = (kind) => kind.replace(/_/g, ' ');
const stepTitle = (step) => {
  const kind = kindOf(step);
  return t(`step.${kind}`) === `step.${kind}` ? pretty(kind) : t(`step.${kind}`);
};

export function stepView(step) {
  const { kind, action, ...rest } = step;
  return h(
    'div',
    { class: 'step' },
    h('strong', {}, stepTitle(step)),
    h(
      'dl',
      { class: 'facts' },
      Object.entries(rest).map(([k, v]) => [
        h('dt', {}, pretty(k)),
        h('dd', {}, Array.isArray(v) && v.some((x) => x && typeof x === 'object' && !Array.isArray(x)) ? h('ul', {}, v.map((x) => h('li', {}, show(x)))) : show(v)),
      ]),
    ),
  );
}

registerPanel({
  id: 'steps',
  title: 'panel.steps',
  appliesTo: ['*'],
  order: 50,
  visible: (tab) => Boolean(tab && tab.steps),
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    let index = 0;
    let timer = null;

    function source() {
      const s = tab.steps && tab.steps.source;
      return s && tab.app.tabs().includes(s) ? s : null;
    }

    const nameIn = (target) => (id) => {
      const state = target && target.isAutomaton ? stateList(target.doc).find((x) => x.id === id) : null;
      return state ? state.name : `#${id}`;
    };

    function highlight() {
      const ids = collectStateIds(tab.steps.steps[index]);
      const from = source();
      const known = (target) => new Set(target && target.isAutomaton ? stateList(target.doc).map((x) => x.id) : []);
      if (from) {
        const have = known(from);
        const list = ids.source.filter((id) => have.has(id));
        from.setOverlay('steps', list.length ? { states: list } : null);
      }
      const mine = known(tab);
      const own = ids.result.filter((id) => mine.has(id));
      tab.setOverlay('steps', own.length ? { states: own } : null);
    }

    function clearHighlight() {
      const from = source();
      if (from) from.setOverlay('steps', null);
      tab.setOverlay('steps', null);
    }

    function go(to) {
      index = Math.max(0, Math.min(tab.steps.steps.length - 1, to));
      draw();
    }

    function stop() {
      if (timer) clearInterval(timer);
      timer = null;
    }

    function play() {
      if (timer) {
        stop();
        draw();
        return;
      }
      timer = setInterval(() => {
        if (index >= tab.steps.steps.length - 1) {
          stop();
          draw();
          return;
        }
        go(index + 1);
      }, 900);
      draw();
    }

    function draw() {
      clear(body);
      if (!tab.steps || !tab.steps.steps.length) {
        body.append(h('p', { class: 'muted' }, t('steps.none')));
        clearHighlight();
        return;
      }
      const all = tab.steps.steps;
      const from = source();
      body.append(
        h('p', { class: 'muted' }, tab.steps.title || ''),
        from ? h('p', { class: 'muted' }, t('steps.source', { title: from.title })) : null,
        h(
          'div',
          { class: 'button-row' },
          h('button', { type: 'button', onclick: () => go(index - 1), disabled: index === 0 }, t('steps.previous')),
          h('span', { role: 'status', 'aria-live': 'polite' }, t('steps.position', { n: index + 1, total: all.length })),
          h('button', { type: 'button', onclick: () => go(index + 1), disabled: index >= all.length - 1 }, t('steps.next')),
          h('button', { type: 'button', onclick: play }, timer ? t('steps.pause') : t('steps.play')),
        ),
        stepView(resolveStep(all[index], nameIn(from), nameIn(tab))),
        h(
          'ol',
          { class: 'step-list' },
          all.map((st, i) =>
            h('li', { class: i === index ? 'current' : '' }, h('button', { type: 'button', class: 'link', 'aria-current': i === index ? 'step' : null, onclick: () => go(i) }, stepTitle(st))),
          ),
        ),
      );
      highlight();
    }

    draw();
    const off = tab.on('steps', () => {
      index = 0;
      draw();
    });
    return {
      destroy() {
        stop();
        clearHighlight();
        off();
      },
    };
  },
});
