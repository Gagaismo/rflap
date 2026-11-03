// The richer views of the conversion replay: the subset table of NFA -> DFA, the distinguishability tree of the
// minimization, and the generalized transition graph of FA -> RE and RE -> NFA. They draw the frames of
// replay-model.js; the graphs are drawn by the engine (`render`), nothing is re-implemented here.

import { h } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { dataTable, svgImage } from './widgets.js';
import { conversionKind, subsetFrames, partitionFrames, prunedTree, treeNodeLabel, gtgFrames, gtgDocument, regexNfaFrames, nfaGraphDocument } from './replay-model.js';
import { layoutTree, treeSvg } from './tree.js';

registerStrings({
  en: {
    'replay.controls': 'Step controls',
    'replay.first': 'First step',
    'replay.previous': 'Previous',
    'replay.next': 'Next',
    'replay.do_all': 'Do all',
    'replay.play': 'Play',
    'replay.pause': 'Pause',
    'replay.position': 'Step {n} of {total}',
    'replay.keys': 'Keys: n / → next, p / ← previous, Home first, End or a do all, Space play.',
    'replay.subset.table': 'DFA states built so far',
    'replay.subset.caption': 'DFA states and the NFA states they stand for',
    'replay.subset.dfa': 'DFA state',
    'replay.subset.nfa': 'NFA states',
    'replay.subset.transitions': 'Transitions so far',
    'replay.subset.no_transitions': 'No transitions yet.',
    'replay.min.partition': 'Groups of equivalent states so far',
    'replay.min.group_split': 'The group {group} is split on {symbol} into {into}.',
    'replay.min.initial': 'Initial partition: final and non-final states.',
    'replay.min.merge': 'The group is merged into the state {state}.',
    'replay.min.tree': 'Distinguishability tree',
    'replay.min.split_on': 'split on {symbol}',
    'replay.gtg.graph': 'Generalized transition graph',
    'replay.gtg.alt': 'Generalized transition graph after step {n}',
    'replay.gtg.edges': 'Labels',
    'replay.gtg.col.from': 'From',
    'replay.gtg.col.label': 'Label',
    'replay.gtg.col.to': 'To',
    'replay.gtg.eliminated': 'State {state} was eliminated.',
    'replay.gtg.empty': '{n} missing transitions were added as ø (not drawn).',
    'replay.gtg.result': 'Regular expression: {expression}',
    'replay.regex.graph': 'Graph after this step',
    'replay.regex.alt': 'Graph of the regular expression after step {n}',
    'replay.regex.replaced': 'Replaced the transition {from} → {to} labeled {label} by:',
    'replay.regex.rendering': 'Drawing…',
    'replay.render_failed': 'The engine could not draw this step: {message}',
  },
  es: {
    'replay.controls': 'Controles de pasos',
    'replay.first': 'Primer paso',
    'replay.previous': 'Anterior',
    'replay.next': 'Siguiente',
    'replay.do_all': 'Hacer todo',
    'replay.play': 'Reproducir',
    'replay.pause': 'Pausar',
    'replay.position': 'Paso {n} de {total}',
    'replay.keys': 'Teclas: n / → siguiente, p / ← anterior, Inicio primero, Fin o a hacer todo, Espacio reproducir.',
    'replay.subset.table': 'Estados del AFD construidos hasta ahora',
    'replay.subset.caption': 'Estados del AFD y los estados del AFN que representan',
    'replay.subset.dfa': 'Estado del AFD',
    'replay.subset.nfa': 'Estados del AFN',
    'replay.subset.transitions': 'Transiciones hasta ahora',
    'replay.subset.no_transitions': 'Todavía no hay transiciones.',
    'replay.min.partition': 'Grupos de estados equivalentes hasta ahora',
    'replay.min.group_split': 'El grupo {group} se divide con {symbol} en {into}.',
    'replay.min.initial': 'Partición inicial: estados finales y no finales.',
    'replay.min.merge': 'El grupo se fusiona en el estado {state}.',
    'replay.min.tree': 'Árbol de distinguibilidad',
    'replay.min.split_on': 'se divide con {symbol}',
    'replay.gtg.graph': 'Grafo de transiciones generalizado',
    'replay.gtg.alt': 'Grafo de transiciones generalizado después del paso {n}',
    'replay.gtg.edges': 'Etiquetas',
    'replay.gtg.col.from': 'Desde',
    'replay.gtg.col.label': 'Etiqueta',
    'replay.gtg.col.to': 'Hasta',
    'replay.gtg.eliminated': 'Se eliminó el estado {state}.',
    'replay.gtg.empty': 'Se agregaron {n} transiciones faltantes como ø (no se dibujan).',
    'replay.gtg.result': 'Expresión regular: {expression}',
    'replay.regex.graph': 'Grafo después de este paso',
    'replay.regex.alt': 'Grafo de la expresión regular después del paso {n}',
    'replay.regex.replaced': 'Se reemplazó la transición {from} → {to} con etiqueta {label} por:',
    'replay.regex.rendering': 'Dibujando…',
    'replay.render_failed': 'El motor no pudo dibujar este paso: {message}',
  },
});

const cache = new WeakMap();

/** `{kind, frames}` of a step list (computed once per list). */
export function framesFor(info) {
  if (!cache.has(info)) {
    const kind = conversionKind(info.steps);
    const frames =
      kind === 'subset' ? subsetFrames(info.steps)
      : kind === 'minimize' ? partitionFrames(info.steps)
      : kind === 'gtg' && info.sourceDoc ? gtgFrames(info.sourceDoc, info.steps)
      : kind === 'regex-nfa' ? regexNfaFrames(info.steps)
      : null;
    cache.set(info, { kind: frames ? kind : null, frames });
  }
  return cache.get(info);
}

const nameFrom = (doc, id) => (doc && doc.states && doc.states[id] ? doc.states[id].name : `#${id}`);
const setText = (ids, nameOf) => `{${ids.map(nameOf).join(', ')}}`;
const edgeText = (e, nameOf) => `${nameOf(e.from)} —${e.label === '' ? 'λ' : e.label}→ ${nameOf(e.to)}`;

async function drawn(tab, document, alt, highlight = []) {
  try {
    const r = await tab.api({ op: 'render', document, format: 'svg', highlight_states: highlight });
    return svgImage(r.content, alt, 'engine-svg step-graph');
  } catch (error) {
    return h('p', { class: 'form-error', role: 'alert' }, t('replay.render_failed', { message: error.message }));
  }
}

function subsetDetail(tab, info, frame) {
  const sourceName = (id) => nameFrom(info.sourceDoc || (info.source && info.source.doc), id);
  const dfaName = (id) => nameFrom(tab.doc, id);
  const hot = new Set(frame.focus.states);
  return h(
    'div',
    { class: 'replay-detail' },
    h('h4', {}, t('replay.subset.table')),
    dataTable(
      t('replay.subset.caption'),
      [t('replay.subset.dfa'), t('replay.subset.nfa')],
      frame.states.map((s) => [{ text: dfaName(s.state), class: 'mono' }, { text: setText(s.nfa, sourceName), class: 'mono' }]),
      { rowClass: (i) => (hot.has(frame.states[i].state) ? 'selected-row' : '') },
    ),
    h('h4', {}, t('replay.subset.transitions')),
    frame.transitions.length
      ? h('ul', { class: 'mono compact-list' }, frame.transitions.map((x) => h('li', { class: hot.has(x.from) && hot.has(x.to) ? 'current' : '' }, `${dfaName(x.from)} —${x.symbol}→ ${dfaName(x.to)}`)))
      : h('p', { class: 'muted' }, t('replay.subset.no_transitions')),
  );
}

function minimizeDetail(tab, info, frame, frames) {
  const nameOf = (id) => nameFrom(info.sourceDoc || (info.source && info.source.doc), id);
  const group = (g) => setText(g, nameOf);
  const step = frame.step;
  const sentence =
    step.kind === 'split' ? t('replay.min.group_split', { group: group(step.group), symbol: step.symbol, into: step.into.map(group).join(', ') })
    : step.kind === 'initial_partition' ? t('replay.min.initial')
    : t('replay.min.merge', { state: nameFrom(tab.doc, step.state) });
  const parts = [
    h('p', { class: 'step-sentence', role: 'status' }, sentence),
    h('h4', {}, t('replay.min.partition')),
    h('ul', { class: 'mono groups' }, frame.groups.map((g) => h('li', { class: frame.changed.some((c) => c.join(',') === g.join(',')) ? 'current' : '' }, group(g)))),
  ];
  if (info.tree) {
    const pruned = prunedTree(info.tree, frame.seen);
    const label = (node) => ({ symbol: treeNodeLabel(node, nameOf), note: node.symbol ? t('replay.min.split_on', { symbol: node.symbol }) : '', children: node.children.map(label) });
    parts.push(h('h4', {}, t('replay.min.tree')), h('div', { class: 'tree-scroll' }, treeSvg(layoutTree(label(pruned)), { label: t('replay.min.tree') })));
  }
  return h('div', { class: 'replay-detail' }, parts);
}

async function gtgDetail(tab, info, frame, index) {
  const nameOf = (id) => nameFrom(info.sourceDoc, id) === `#${id}` ? `q${id}` : nameFrom(info.sourceDoc, id);
  const graph = await drawn(tab, gtgDocument(info.sourceDoc, frame), t('replay.gtg.alt', { n: index + 1 }));
  const notes = [];
  if (frame.eliminated !== null) notes.push(h('p', { role: 'status' }, t('replay.gtg.eliminated', { state: nameOf(frame.eliminated) })));
  if (frame.emptyPairs) notes.push(h('p', { class: 'muted' }, t('replay.gtg.empty', { n: frame.emptyPairs })));
  if (frame.expression !== null) notes.push(h('p', { class: 'mono big-form', role: 'status' }, t('replay.gtg.result', { expression: frame.expression })));
  return h(
    'div',
    { class: 'replay-detail' },
    h('h4', {}, t('replay.gtg.graph')),
    graph,
    notes,
    h('h4', {}, t('replay.gtg.edges')),
    dataTable(t('replay.gtg.edges'), [t('replay.gtg.col.from'), t('replay.gtg.col.label'), t('replay.gtg.col.to')], frame.edges.map((e) => [nameOf(e.from), { text: e.label, class: 'mono' }, nameOf(e.to)])),
  );
}

async function regexDetail(tab, frame, index) {
  const document = nfaGraphDocument(tab.doc, frame);
  const graph = await drawn(tab, document, t('replay.regex.alt', { n: index + 1 }), frame.addedStates);
  return h(
    'div',
    { class: 'replay-detail' },
    h('h4', {}, t('replay.regex.graph')),
    graph,
    h('p', { class: 'muted' }, t('replay.regex.replaced', { from: nameFrom(tab.doc, frame.replaced.from), to: nameFrom(tab.doc, frame.replaced.to), label: frame.replaced.label })),
    h('ul', { class: 'mono compact-list' }, frame.addedEdges.map((e) => h('li', {}, edgeText(e, (id) => nameFrom(tab.doc, id))))),
  );
}

/**
 * The detail below a step: a node, or null when the steps have no richer view. `info` is `tab.steps`
 * (`{steps, source, to, tree, sourceDoc}`); `index` the current step.
 */
export async function conversionDetail(tab, info, index) {
  const { kind, frames } = framesFor(info);
  if (!kind || !frames[index]) return null;
  const frame = frames[index];
  if (kind === 'subset') return subsetDetail(tab, info, frame);
  if (kind === 'minimize') return minimizeDetail(tab, info, frame, frames);
  if (kind === 'gtg') return gtgDetail(tab, info, frame, index);
  if (kind === 'regex-nfa' && tab.type === 'fa') return regexDetail(tab, frame, index);
  return null;
}
