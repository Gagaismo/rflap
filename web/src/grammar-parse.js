// Grammar environment, part 4: the parsers (brute force, LL(1), SLR(1), CYK) with their traces, the parse
// tree and the derivation, the multiple-input table, and the user-controlled parse.

import { h, clear } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { dataTable, outcomeBadge, withLambda } from './widgets.js';
import { Replay, replayBar, replayKeys } from './replay.js';
import { layoutTree, treeSvg, treeYield } from './tree.js';
import { ll1Trace, slrTrace, cykModel, derivationSteps, userMove, withoutLastMove, userChoices, parseInputLines } from './grammar-model.js';
import { guarded } from './actions.js';

registerStrings({
  en: {
    'panel.grammar_parse': 'Parse',
    'panel.grammar_user': 'User parse',
    'parse.inputs': 'Strings to parse (one per line, λ or an empty line for the empty string)',
    'parse.parser': 'Parser',
    'parse.parser.brute': 'Brute force',
    'parse.parser.ll1': 'LL(1)',
    'parse.parser.slr1': 'SLR(1)',
    'parse.parser.cyk': 'CYK (needs Chomsky normal form)',
    'parse.run': 'Parse',
    'parse.budget': 'Brute force search limit (forms examined)',
    'parse.results': 'Results',
    'parse.col.input': 'String',
    'parse.col.outcome': 'Result',
    'parse.col.detail': 'Detail',
    'parse.show': 'Show',
    'parse.outcome.accepted': 'Accepted',
    'parse.outcome.rejected': 'Rejected',
    'parse.outcome.error': 'Cannot parse',
    'parse.trace': 'Trace',
    'parse.col.n': '#',
    'parse.col.stack': 'Stack',
    'parse.col.states': 'States',
    'parse.col.symbols': 'Symbols',
    'parse.col.input_left': 'Input',
    'parse.col.action': 'Action',
    'parse.derivation': 'Derivation',
    'parse.derivation_none': 'No derivation: the string was not accepted.',
    'parse.col.production': 'Production',
    'parse.col.form': 'Sentential form',
    'parse.tree': 'Parse tree',
    'parse.tree_none': 'No parse tree.',
    'parse.tree_yield': 'Yield: {text}',
    'parse.cyk': 'CYK table',
    'parse.cyk_hint': 'Row i, column j: the variables that derive the substring from position i to j.',
    'parse.examined': 'Forms examined: {n}.',
    'parse.empty': 'Write at least one string (an empty line parses the empty string).',
    'parse.error': 'Error: {message}',
    'parse.user.target': 'String to derive',
    'parse.user.start': 'Start',
    'parse.user.form': 'Sentential form',
    'parse.user.choices': 'Productions you can apply',
    'parse.user.none': 'No production applies to the current form.',
    'parse.user.apply': 'Apply {production} ({label})',
    'parse.user.all': 'Apply to all {n} occurrences',
    'parse.user.undo': 'Undo',
    'parse.user.reset': 'Start over',
    'parse.user.done': 'Done: the form is the string.',
    'parse.user.moves': 'Moves: {n}',
    'parse.user.error': 'That move is not allowed: {message}',
  },
  es: {
    'panel.grammar_parse': 'Análisis',
    'panel.grammar_user': 'Análisis manual',
    'parse.inputs': 'Cadenas a analizar (una por línea, λ o una línea vacía para la cadena vacía)',
    'parse.parser': 'Analizador',
    'parse.parser.brute': 'Fuerza bruta',
    'parse.parser.ll1': 'LL(1)',
    'parse.parser.slr1': 'SLR(1)',
    'parse.parser.cyk': 'CYK (necesita forma normal de Chomsky)',
    'parse.run': 'Analizar',
    'parse.budget': 'Límite de la búsqueda por fuerza bruta (formas examinadas)',
    'parse.results': 'Resultados',
    'parse.col.input': 'Cadena',
    'parse.col.outcome': 'Resultado',
    'parse.col.detail': 'Detalle',
    'parse.show': 'Ver',
    'parse.outcome.accepted': 'Aceptada',
    'parse.outcome.rejected': 'Rechazada',
    'parse.outcome.error': 'No se puede analizar',
    'parse.trace': 'Traza',
    'parse.col.n': '#',
    'parse.col.stack': 'Pila',
    'parse.col.states': 'Estados',
    'parse.col.symbols': 'Símbolos',
    'parse.col.input_left': 'Entrada',
    'parse.col.action': 'Acción',
    'parse.derivation': 'Derivación',
    'parse.derivation_none': 'No hay derivación: la cadena no fue aceptada.',
    'parse.col.production': 'Producción',
    'parse.col.form': 'Forma sentencial',
    'parse.tree': 'Árbol de derivación',
    'parse.tree_none': 'No hay árbol de derivación.',
    'parse.tree_yield': 'Frontera: {text}',
    'parse.cyk': 'Tabla CYK',
    'parse.cyk_hint': 'Fila i, columna j: las variables que derivan la subcadena de la posición i a la j.',
    'parse.examined': 'Formas examinadas: {n}.',
    'parse.empty': 'Escribí al menos una cadena (una línea vacía analiza la cadena vacía).',
    'parse.error': 'Error: {message}',
    'parse.user.target': 'Cadena a derivar',
    'parse.user.start': 'Empezar',
    'parse.user.form': 'Forma sentencial',
    'parse.user.choices': 'Producciones que podés aplicar',
    'parse.user.apply': 'Aplicar {production} ({label})',
    'parse.user.none': 'Ninguna producción se aplica a la forma actual.',
    'parse.user.all': 'Aplicar a las {n} apariciones',
    'parse.user.undo': 'Deshacer',
    'parse.user.reset': 'Empezar de nuevo',
    'parse.user.done': 'Listo: la forma es la cadena.',
    'parse.user.moves': 'Movimientos: {n}',
    'parse.user.error': 'Ese movimiento no está permitido: {message}',
  },
});

const PARSERS = ['brute', 'll1', 'slr1', 'cyk'];

function treeBlock(tree, derivation) {
  if (!tree) return h('p', { class: 'muted' }, t('parse.tree_none'));
  const label = derivation && derivation.length ? derivation.map((d) => d.form).join(' ⇒ ') : t('parse.tree');
  return h('div', { class: 'tree-box' }, h('div', { class: 'tree-scroll' }, treeSvg(layoutTree(tree), { label })), h('p', { class: 'muted mono' }, t('parse.tree_yield', { text: treeYield(tree) || 'λ' })));
}

function derivationBlock(result) {
  const steps = derivationSteps(result);
  if (!steps.length) return h('p', { class: 'muted' }, t('parse.derivation_none'));
  return dataTable(
    t('parse.derivation'),
    [t('parse.col.n'), t('parse.col.production'), t('parse.col.form')],
    [[0, '', { text: result.derivation.start, class: 'mono' }], ...steps.map((s) => [s.n, { text: s.production, class: 'mono' }, { text: s.form === '' ? 'λ' : s.form, class: 'mono' }])],
  );
}

function cykBlock(result) {
  const model = cykModel(result.table, result.input);
  return h(
    'div',
    {},
    h('h4', {}, t('parse.cyk')),
    h('p', { class: 'muted' }, t('parse.cyk_hint')),
    dataTable(
      t('parse.cyk'),
      ['', ...model.header.map((c, j) => `${j + 1}:${c}`)],
      model.rows.map((row) => [
        `${row.i + 1}:${model.header[row.i]}`,
        ...model.header.map((_, j) => {
          const cell = row.cells.find((c) => c.j === j);
          return cell ? { text: cell.symbols.length ? `{${cell.symbols.join(',')}}` : '∅', class: 'mono', title: cell.substring } : '';
        }),
      ]),
    ),
  );
}

registerPanel({
  id: 'grammar-parse',
  title: 'panel.grammar_parse',
  appliesTo: ['grammar'],
  order: 40,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    const inputs = h('textarea', { rows: 3, class: 'mono', spellcheck: 'false', 'aria-label': t('parse.inputs'), value: '' });
    const parser = h('select', { 'aria-label': t('parse.parser') }, PARSERS.map((p) => h('option', { value: p }, t(`parse.parser.${p}`))));
    const budget = h('input', { type: 'number', min: 1, value: '20000', class: 'narrow', 'aria-label': t('parse.budget') });
    const output = h('div', { class: 'parse-output', 'aria-live': 'polite' });
    let answer = null; // { parser, results }
    let selected = 0;
    const replay = new Replay(0);
    let bar = null;

    async function run() {
      const list = parseInputLines(inputs.value);
      await guarded(tab, async () => {
        const request = { op: 'parse', grammar: tab.doc, inputs: list, parser: parser.value };
        if (parser.value === 'brute') request.budget = Math.max(1, Number(budget.value) || 20000);
        answer = await tab.api(request);
        selected = 0;
        showDetail();
      });
    }

    function showDetail() {
      if (bar) bar.stop();
      clear(output);
      if (!answer) return;
      const results = answer.results;
      output.append(
        h('h3', {}, t('parse.results')),
        dataTable(
          t('parse.results'),
          [t('parse.col.input'), t('parse.col.outcome'), ''],
          results.map((r, i) => [
            { text: r.input === '' ? 'λ' : r.input, class: 'mono' },
            outcomeBadge(r.outcome),
            h('button', { type: 'button', 'aria-pressed': String(i === selected), onclick: () => ((selected = i), replay.reset(0), showDetail()) }, t('parse.show')),
          ]),
          { rowClass: (i) => (i === selected ? 'selected-row' : '') },
        ),
      );
      const r = results[selected];
      if (!r) return;
      output.append(h('h3', {}, r.input === '' ? 'λ' : r.input), outcomeBadge(r.outcome));
      if (r.outcome === 'error') {
        output.append(h('p', { class: 'form-error', role: 'alert' }, t('parse.error', { message: r.error.message })));
        return;
      }
      if (r.candidates_examined !== undefined) output.append(h('p', { class: 'muted' }, t('parse.examined', { n: r.candidates_examined })));
      const rows = answer.parser === 'll1' ? ll1Trace(r) : answer.parser === 'slr1' ? slrTrace(r) : [];
      if (rows.length) {
        if (replay.count !== rows.length) {
          replay.reset(rows.length);
          replay.last();
        }
        bar = replayBar(replay, showDetail, { key: `parse-${tab.id}` });
        const head = answer.parser === 'll1' ? [t('parse.col.n'), t('parse.col.stack'), t('parse.col.input_left'), t('parse.col.action')] : [t('parse.col.n'), t('parse.col.states'), t('parse.col.symbols'), t('parse.col.input_left'), t('parse.col.action')];
        const shown = rows.slice(0, replay.index + 1);
        const region = h(
          'div',
          { class: 'replay-region', 'aria-label': t('parse.trace') },
          h('h4', {}, t('parse.trace')),
          bar.node,
          dataTable(t('parse.trace'), head, shown.map((row) => (answer.parser === 'll1' ? [row.n, { text: row.stack || 'λ', class: 'mono' }, { text: row.input, class: 'mono' }, { text: row.action, class: `action-${row.kind}` }] : [row.n, { text: row.states, class: 'mono' }, { text: row.symbols || 'λ', class: 'mono' }, { text: row.input, class: 'mono' }, { text: row.action, class: `action-${row.kind}` }])), { rowClass: (i) => (i === replay.index ? 'selected-row' : '') }),
        );
        replayKeys(region, bar.act);
        output.append(region);
      }
      if (answer.parser === 'cyk' && r.table) output.append(cykBlock(r));
      output.append(h('h4', {}, t('parse.derivation')), derivationBlock(r), h('h4', {}, t('parse.tree')), treeBlock(r.tree, derivationSteps(r)));
    }

    const setParser = (name) => () => {
      parser.value = name;
      inputs.focus();
    };
    body.append(
      h('div', { class: 'form-row' }, h('label', { for: `pin-${tab.id}` }, t('parse.inputs')), withLambda(Object.assign(inputs, { id: `pin-${tab.id}` }))),
      h('div', { class: 'sim-options' }, h('label', { class: 'inline' }, t('parse.parser'), parser), h('label', { class: 'inline' }, t('parse.budget'), budget)),
      h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: run }, t('parse.run'))),
      output,
    );
    inputs.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        run();
      }
    });
    const off = tab.on('doc', (e) => {
      if (e.cause === 'edit' || e.cause === 'undo' || e.cause === 'redo') {
        answer = null;
        clear(output);
      }
    });
    return { destroy: () => (bar && bar.stop(), off()), focus: () => inputs.focus(), useBrute: setParser('brute'), useLL1: setParser('ll1'), useSLR1: setParser('slr1'), useCYK: setParser('cyk') };
  },
});

registerPanel({
  id: 'grammar-user',
  title: 'panel.grammar_user',
  appliesTo: ['grammar'],
  order: 50,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    const target = h('input', { type: 'text', class: 'mono', id: `utarget-${tab.id}`, spellcheck: 'false', value: '' });
    let moves = [];
    let state = null; // the last answer of user_parse
    let problem = '';

    async function refresh(nextMoves) {
      problem = '';
      try {
        const text = target.value.trim() === 'λ' ? '' : target.value.trim();
        state = await tab.api({ op: 'user_parse', grammar: tab.doc, target: text, moves: nextMoves });
        moves = nextMoves;
      } catch (error) {
        problem = error.code === 'invalid_move' ? t('parse.user.error', { message: error.message }) : error.message;
        if (!state) moves = [];
      }
      draw();
    }

    function choiceButtons(choice) {
      const buttons = choice.positions.map((position, i) =>
        h('button', { type: 'button', title: choice.text, onclick: () => refresh([...moves, userMove({ production: choice.production, position })]) }, t('parse.user.apply', { production: choice.text, label: choice.labels[i] })),
      );
      if (choice.all) buttons.push(h('button', { type: 'button', onclick: () => refresh([...moves, userMove({ production: choice.production, positions: choice.positions })]) }, t('parse.user.all', { n: choice.positions.length })));
      return h('li', {}, buttons);
    }

    function draw() {
      clear(body);
      body.append(
        h('div', { class: 'form-row' }, h('label', { for: target.id }, t('parse.user.target')), withLambda(target)),
        h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: () => refresh([]) }, state ? t('parse.user.reset') : t('parse.user.start')), h('button', { type: 'button', disabled: !moves.length, onclick: () => refresh(withoutLastMove(moves)) }, t('parse.user.undo'))),
      );
      if (problem) body.append(h('p', { class: 'form-error', role: 'alert' }, problem));
      if (!state) return;
      const choices = userChoices(state);
      body.append(
        h('h3', {}, t('parse.user.form')),
        h('p', { class: 'mono big-form', role: 'status' }, state.sentential === '' ? 'λ' : state.sentential),
        h('p', { class: 'muted' }, t('parse.user.moves', { n: moves.length })),
      );
      if (state.finished) body.append(h('p', { class: 'status-ok', role: 'status' }, t('parse.user.done')));
      else {
        body.append(h('h3', {}, t('parse.user.choices')), choices.length ? h('ul', { class: 'choices' }, choices.map(choiceButtons)) : h('p', { class: 'muted' }, t('parse.user.none')));
      }
      body.append(h('h3', {}, t('parse.derivation')), derivationBlock({ derivation: state.derivation, ...state }), h('h3', {}, t('parse.tree')), treeBlock(state.tree, derivationSteps({ derivation: state.derivation })));
    }

    draw();
    const off = tab.on('doc', (e) => {
      if (e.cause === 'edit' || e.cause === 'undo' || e.cause === 'redo') {
        state = null;
        moves = [];
        draw();
      }
    });
    return { destroy: off, focus: () => target.focus() };
  },
});

const PARSE_ACTIONS = [
  ['brute', 'parse.parser.brute', 'useBrute', 10],
  ['ll1', 'parse.parser.ll1', 'useLL1', 11],
  ['slr1', 'parse.parser.slr1', 'useSLR1', 12],
  ['cyk', 'parse.parser.cyk', 'useCYK', 13],
];
for (const [id, label, method, order] of PARSE_ACTIONS) {
  registerAction({ id: `input.parse.${id}`, menu: 'input', label, appliesTo: ['grammar'], order, run: (tab) => tab.app.showPanel(tab, 'grammar-parse', method) });
}
registerAction({ id: 'input.parse.user', menu: 'input', label: 'panel.grammar_user', appliesTo: ['grammar'], order: 14, run: (tab) => tab.app.showPanel(tab, 'grammar-user') });
