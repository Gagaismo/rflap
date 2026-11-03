// Grammar environment, part 1: the production table editor (add / remove / reorder / λ / paste text) and the
// type test. Documents of type `grammar` are `{type, productions: [{lhs, rhs}], start}`.

import { h, clear, debounce, put } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { withLambda, statusLine } from './widgets.js';
import {
  parseProductionText, serializeProductions, addGrammarRow, removeGrammarRow, moveGrammarRow, setGrammarRow, startVariable,
} from './grammar-model.js';

registerStrings({
  en: {
    'panel.grammar_editor': 'Grammar',
    'file.new_grammar': 'New grammar',
    'grammar.start': 'Start variable',
    'grammar.start_hint': 'Empty means the left side of the first production.',
    'grammar.productions': 'Productions',
    'grammar.col.lhs': 'Left side',
    'grammar.col.rhs': 'Right side',
    'grammar.lhs_of': 'Left side of production {n}',
    'grammar.rhs_of': 'Right side of production {n} (empty or λ is the empty string)',
    'grammar.add': 'Add production',
    'grammar.remove': 'Remove production {n}',
    'grammar.up': 'Move production {n} up',
    'grammar.down': 'Move production {n} down',
    'grammar.empty': 'The grammar has no productions yet.',
    'grammar.text': 'As text',
    'grammar.text_hint': 'One line per variable: S -> aSb | λ. Paste a grammar here and apply it.',
    'grammar.apply_text': 'Replace the table with this text',
    'grammar.text_applied': 'Productions read: {n}.',
    'grammar.rejected': 'The engine did not accept the grammar: {message}',
    'grammar.type': 'Grammar type',
    'grammar.type.checking': 'Checking…',
    'grammar.type.flags': 'Regular (right-linear): {right}. Left-linear: {left}. Context-free: {cf}. Context-sensitive: {cs}. Chomsky normal form: {cnf}. Greibach normal form: {gnf}.',
  },
  es: {
    'panel.grammar_editor': 'Gramática',
    'file.new_grammar': 'Nueva gramática',
    'grammar.start': 'Variable inicial',
    'grammar.start_hint': 'Vacío significa el lado izquierdo de la primera producción.',
    'grammar.productions': 'Producciones',
    'grammar.col.lhs': 'Lado izquierdo',
    'grammar.col.rhs': 'Lado derecho',
    'grammar.lhs_of': 'Lado izquierdo de la producción {n}',
    'grammar.rhs_of': 'Lado derecho de la producción {n} (vacío o λ es la cadena vacía)',
    'grammar.add': 'Agregar producción',
    'grammar.remove': 'Quitar la producción {n}',
    'grammar.up': 'Subir la producción {n}',
    'grammar.down': 'Bajar la producción {n}',
    'grammar.empty': 'La gramática todavía no tiene producciones.',
    'grammar.text': 'Como texto',
    'grammar.text_hint': 'Una línea por variable: S -> aSb | λ. Pegá una gramática acá y aplicala.',
    'grammar.apply_text': 'Reemplazar la tabla con este texto',
    'grammar.text_applied': 'Producciones leídas: {n}.',
    'grammar.rejected': 'El motor no aceptó la gramática: {message}',
    'grammar.type': 'Tipo de gramática',
    'grammar.type.checking': 'Comprobando…',
    'grammar.type.flags': 'Regular (lineal por la derecha): {right}. Lineal por la izquierda: {left}. Libre de contexto: {cf}. Sensible al contexto: {cs}. Forma normal de Chomsky: {cnf}. Forma normal de Greibach: {gnf}.',
  },
});

const flagWord = (value) => (value ? t('word.yes') : t('word.no'));

registerPanel({
  id: 'grammar-editor',
  title: 'panel.grammar_editor',
  appliesTo: ['grammar'],
  order: 10,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body grammar-editor' });
    root.append(body);
    let rows = tab.doc.productions.map((p) => ({ ...p }));
    let start = tab.doc.start || '';
    let problem = '';
    let notice = '';
    let typeBox = null;
    let busy = false;

    /** Sends the table to the engine; only a grammar it accepts becomes the document (and an undo step). */
    async function commit(nextRows, nextStart = start) {
      rows = nextRows;
      start = nextStart;
      problem = '';
      busy = true;
      try {
        const productions = rows.filter((r) => r.lhs !== '');
        const doc = { type: 'grammar', productions, start: startVariable(productions, start) };
        const { document } = await tab.api({ op: 'normalize', document: doc });
        await tab.commit(document);
      } catch (error) {
        problem = t('grammar.rejected', { message: error.message });
        draw();
      } finally {
        busy = false;
      }
    }

    const field = (value, label, onchange, extra = {}) =>
      h('input', { type: 'text', value, class: 'narrow mono', 'aria-label': label, spellcheck: 'false', autocomplete: 'off', onchange: (e) => onchange(e.target.value), ...extra });

    function draw() {
      const focused = document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.cell : null;
      clear(body);
      const table = h(
        'table',
        { class: 'configs production-table' },
        h('caption', { class: 'sr-only' }, t('grammar.productions')),
        h('thead', {}, h('tr', {}, h('th', {}, '#'), h('th', {}, t('grammar.col.lhs')), h('th', {}), h('th', {}, t('grammar.col.rhs')), h('th', {}))),
        h(
          'tbody',
          {},
          rows.map((row, i) => {
            const n = i + 1;
            const lhs = field(row.lhs, t('grammar.lhs_of', { n }), (v) => commit(setGrammarRow(rows, i, { lhs: v })), { dataset: { cell: `l${i}` } });
            const rhs = field(row.rhs === '' ? 'λ' : row.rhs, t('grammar.rhs_of', { n }), (v) => commit(setGrammarRow(rows, i, { rhs: v })), { dataset: { cell: `r${i}` } });
            return h(
              'tr',
              {},
              h('td', {}, n),
              h('td', {}, lhs),
              h('td', { 'aria-hidden': 'true' }, '→'),
              h('td', {}, withLambda(rhs)),
              h(
                'td',
                { class: 'row-buttons' },
                h('button', { type: 'button', 'aria-label': t('grammar.up', { n }), title: t('grammar.up', { n }), disabled: i === 0, onclick: () => commit(moveGrammarRow(rows, i, -1)) }, '↑'),
                h('button', { type: 'button', 'aria-label': t('grammar.down', { n }), title: t('grammar.down', { n }), disabled: i === rows.length - 1, onclick: () => commit(moveGrammarRow(rows, i, 1)) }, '↓'),
                h('button', { type: 'button', 'aria-label': t('grammar.remove', { n }), title: t('grammar.remove', { n }), onclick: () => commit(removeGrammarRow(rows, i)) }, '✕'),
              ),
            );
          }),
        ),
      );
      const text = h('textarea', { rows: 5, class: 'mono', spellcheck: 'false', 'aria-label': t('grammar.text'), value: serializeProductions(rows) });
      const startInput = h('input', { type: 'text', value: start, class: 'narrow mono', id: `gstart-${tab.id}`, onchange: (e) => commit(rows, e.target.value.trim()) });
      typeBox = h('div', { class: 'grammar-type', 'aria-live': 'polite' }, t('grammar.type.checking'));
      put(
        body,
        h('div', { class: 'form-row' }, h('label', { for: `gstart-${tab.id}` }, t('grammar.start')), startInput, h('small', {}, t('grammar.start_hint'))),
        h('h3', {}, t('grammar.productions')),
        rows.length ? table : h('p', { class: 'muted' }, t('grammar.empty')),
        h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => {
            rows = addGrammarRow(rows);
            draw();
            focusCell(`l${rows.length - 1}`);
          } }, t('grammar.add'))),
        problem ? h('p', { class: 'form-error', role: 'alert' }, problem) : null,
        h('h3', {}, t('grammar.text')),
        h('p', { class: 'muted' }, t('grammar.text_hint')),
        text,
        h(
          'div',
          { class: 'button-row' },
          h('button', { type: 'button', onclick: () => {
            const parsed = parseProductionText(text.value);
            if (parsed.errors.length) {
              problem = parsed.errors.map((e) => e.message).join(' ');
              notice = '';
              draw();
              return;
            }
            notice = t('grammar.text_applied', { n: parsed.rows.length });
            commit(parsed.rows);
          } }, t('grammar.apply_text')),
        ),
        notice ? h('p', { class: 'muted', role: 'status' }, notice) : null,
        h('h3', {}, t('grammar.type')),
        typeBox,
      );
      if (focused) focusCell(focused);
      showType();
    }

    function focusCell(name) {
      const node = body.querySelector(`[data-cell="${name}"]`);
      if (node) node.focus();
    }

    let typeToken = 0;
    async function showType() {
      const token = ++typeToken;
      const box = typeBox;
      try {
        const r = await tab.api({ op: 'grammar_type', grammar: tab.doc });
        if (token !== typeToken || box !== typeBox) return;
        const flags = r.report;
        clear(box).append(
          h('p', {}, h('strong', {}, r.message)),
          h('p', { class: 'muted' }, t('grammar.type.flags', { right: flagWord(flags.right_linear), left: flagWord(flags.left_linear), cf: flagWord(flags.context_free), cs: flagWord(flags.context_sensitive), cnf: flagWord(flags.cnf), gnf: flagWord(flags.gnf) })),
        );
      } catch (error) {
        if (token === typeToken && box === typeBox) clear(box).append(statusLine(false, '', error.message));
      }
    }

    const redraw = debounce(() => {
      if (busy) return;
      rows = [...tab.doc.productions.map((p) => ({ ...p })), ...rows.filter((r) => r.lhs === '')]; // rows still being typed stay
      start = tab.doc.start || '';
      problem = '';
      draw();
    }, 0);
    draw();
    const off = tab.on('doc', redraw);
    return { destroy: off, focus: () => focusCell('l0') };
  },
});

registerAction({
  id: 'file.new.grammar',
  menu: 'file',
  label: 'file.new_grammar',
  order: 5,
  run: (tab) => tab.app.newTab({ type: 'grammar', productions: [], start: '' }),
});
