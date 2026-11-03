// Step viewer for the `steps` list a conversion returns (subset construction, minimization, state
// elimination, ...). Each step is shown readably and the states it mentions are highlighted: ids of the
// automaton that was converted on the canvas of its tab, ids of the result on this tab's canvas.

import { h, clear, put } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { stateList } from './doc.js';
import { Replay, replayBar, replayKeys } from './replay.js';
import { conversionDetail } from './replay-views.js';

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
  if (typeof value === 'object' && 'from' in value && 'to' in value && 'label' in value) return `${show(value.from)} —${show(value.label)}→ ${show(value.to)}`;
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
    const replay = new Replay(tab.steps ? tab.steps.steps.length : 0);
    let bar = null;
    let drawing = 0;

    function source() {
      const s = tab.steps && tab.steps.source;
      return s && tab.app.tabs().includes(s) ? s : null;
    }

    const nameIn = (target) => (id) => {
      const state = target && target.isAutomaton ? stateList(target.doc).find((x) => x.id === id) : null;
      return state ? state.name : `#${id}`;
    };

    function highlight() {
      const ids = collectStateIds(tab.steps.steps[replay.index]);
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

    function draw() {
      if (bar) bar.stop();
      const mine = ++drawing;
      clear(body);
      if (!tab.steps || !tab.steps.steps.length) {
        body.append(h('p', { class: 'muted' }, t('steps.none')));
        clearHighlight();
        return;
      }
      const all = tab.steps.steps;
      const from = source();
      bar = replayBar(replay, draw, { key: `steps-${tab.id}` });
      const detail = h('div', { class: 'replay-extra' });
      const region = h(
        'div',
        { class: 'replay-region', 'aria-label': t('panel.steps') },
        bar.node,
        stepView(resolveStep(all[replay.index], nameIn(from), nameIn(tab))),
        detail,
        h('p', { class: 'muted' }, t('replay.keys')),
      );
      put(
        body,
        h('p', { class: 'muted' }, tab.steps.title || ''),
        from ? h('p', { class: 'muted' }, t('steps.source', { title: from.title })) : null,
        region,
        h(
          'ol',
          { class: 'step-list' },
          all.map((st, i) =>
            h('li', { class: i === replay.index ? 'current' : '' }, h('button', { type: 'button', class: 'link', 'aria-current': i === replay.index ? 'step' : null, onclick: () => (replay.goTo(i), draw()) }, stepTitle(st))),
          ),
        ),
      );
      replayKeys(region, bar.act);
      highlight();
      conversionDetail(tab, tab.steps, replay.index)
        .then((node) => {
          if (node && mine === drawing) detail.append(node);
        })
        .catch(() => {
          /* the generic step view above already says everything the steps carry */
        });
    }

    draw();
    const off = tab.on('steps', () => {
      replay.reset(tab.steps ? tab.steps.steps.length : 0);
      draw();
    });
    return {
      destroy() {
        if (bar) bar.stop();
        clearHighlight();
        off();
      },
    };
  },
});
