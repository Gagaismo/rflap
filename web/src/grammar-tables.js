// Grammar environment, part 3: FIRST / FOLLOW, the LL(1) table and the SLR(1) items, ACTION and GOTO tables,
// with the conflicts highlighted (JFLAP's "LL(1) Parse" and "SLR(1) Parse" tables).

import { h, clear } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { dataTable, statusLine } from './widgets.js';
import { tablesModel } from './grammar-model.js';
import { guarded } from './actions.js';

registerStrings({
  en: {
    'panel.grammar_tables': 'Tables',
    'tables.compute': 'Compute the tables',
    'tables.first_follow': 'FIRST and FOLLOW',
    'tables.col.variable': 'Variable',
    'tables.col.first': 'FIRST',
    'tables.col.follow': 'FOLLOW',
    'tables.ll1': 'LL(1) table',
    'tables.ll1.ok': 'The grammar is LL(1): no cell has more than one production.',
    'tables.ll1.bad': 'The grammar is not LL(1): the highlighted cells have several productions.',
    'tables.slr': 'SLR(1) parse table',
    'tables.slr.ok': 'The grammar is SLR(1): no ACTION cell has a conflict.',
    'tables.slr.bad': 'The grammar is not SLR(1): the highlighted ACTION cells have conflicts.',
    'tables.productions': 'Numbered productions',
    'tables.items': 'Items of each state',
    'tables.state': 'State',
    'tables.items_of': 'Items',
    'tables.goes': 'Transitions',
    'tables.action': 'ACTION',
    'tables.goto': 'GOTO',
    'tables.conflict': 'conflict',
    'tables.conflicts': 'Conflicts',
    'tables.ll1_conflict': 'In row {variable}, column {terminal}: {entries}.',
    'tables.slr_conflict': 'In state {state} on {symbol}: {actions}.',
    'tables.final_state': 'accepting item state',
    'tables.empty': 'The grammar has no productions.',
  },
  es: {
    'panel.grammar_tables': 'Tablas',
    'tables.compute': 'Calcular las tablas',
    'tables.first_follow': 'PRIMEROS y SIGUIENTES',
    'tables.col.variable': 'Variable',
    'tables.col.first': 'PRIMEROS',
    'tables.col.follow': 'SIGUIENTES',
    'tables.ll1': 'Tabla LL(1)',
    'tables.ll1.ok': 'La gramática es LL(1): ninguna celda tiene más de una producción.',
    'tables.ll1.bad': 'La gramática no es LL(1): las celdas resaltadas tienen varias producciones.',
    'tables.slr': 'Tabla de análisis SLR(1)',
    'tables.slr.ok': 'La gramática es SLR(1): ninguna celda de ACCIÓN tiene conflicto.',
    'tables.slr.bad': 'La gramática no es SLR(1): las celdas de ACCIÓN resaltadas tienen conflictos.',
    'tables.productions': 'Producciones numeradas',
    'tables.items': 'Ítems de cada estado',
    'tables.state': 'Estado',
    'tables.items_of': 'Ítems',
    'tables.goes': 'Transiciones',
    'tables.action': 'ACCIÓN',
    'tables.goto': 'IR_A',
    'tables.conflict': 'conflicto',
    'tables.conflicts': 'Conflictos',
    'tables.ll1_conflict': 'En la fila {variable}, columna {terminal}: {entries}.',
    'tables.slr_conflict': 'En el estado {state} con {symbol}: {actions}.',
    'tables.final_state': 'estado con ítem de aceptación',
    'tables.empty': 'La gramática no tiene producciones.',
  },
});

const cellView = (cell) => ({ text: cell.entries.join(' / '), class: cell.conflict ? 'conflict' : '', title: cell.conflict ? t('tables.conflict') : undefined });

/** The three tables as DOM; conflicts get the `conflict` class and a text marker (not only a colour). */
export function tablesView(model) {
  const parts = [];
  parts.push(
    h('h3', {}, t('tables.first_follow')),
    dataTable(t('tables.first_follow'), [t('tables.col.variable'), t('tables.col.first'), t('tables.col.follow')], model.firstFollow.map((r) => [{ text: r.variable, class: 'mono' }, { text: `{ ${r.first.join(', ')} }`, class: 'mono' }, { text: `{ ${r.follow.join(', ')} }`, class: 'mono' }])),
  );
  const ll = model.ll1;
  parts.push(
    h('h3', {}, t('tables.ll1')),
    statusLine(ll.ok, t('tables.ll1.ok'), t('tables.ll1.bad')),
    dataTable(t('tables.ll1'), ['', ...ll.terminals], ll.rows.map((r) => [{ text: r.variable, class: 'mono' }, ...r.cells.map(cellView)])),
  );
  if (ll.conflicts.length) {
    parts.push(h('h4', {}, t('tables.conflicts')), h('ul', {}, ll.conflicts.map((c) => h('li', { class: 'mono' }, t('tables.ll1_conflict', { variable: c.variable, terminal: c.terminal, entries: c.entries.join(' / ') })))));
  }
  const lr = model.slr1;
  parts.push(
    h('h3', {}, t('tables.slr')),
    statusLine(lr.ok, t('tables.slr.ok'), t('tables.slr.bad')),
    h('h4', {}, t('tables.productions')),
    h('ol', { start: 0, class: 'mono' }, lr.productions.map((p) => h('li', {}, p.text))),
    h('h4', {}, t('tables.items')),
    dataTable(t('tables.items'), [t('tables.state'), t('tables.items_of'), t('tables.goes')], lr.states.map((s) => [
      { text: `${s.id}${s.final ? ' ✓' : ''}`, title: s.final ? t('tables.final_state') : undefined },
      h('div', { class: 'mono' }, s.items.map((item) => h('div', {}, item))),
      { text: Object.entries(s.transitions).map(([symbol, to]) => `${symbol}→${to}`).join('  '), class: 'mono' },
    ])),
    h('h4', {}, `${t('tables.action')} / ${t('tables.goto')}`),
    dataTable(
      `${t('tables.action')} / ${t('tables.goto')}`,
      [t('tables.state'), ...lr.terminals, ...lr.variables],
      lr.actionRows.map((row, i) => [row.state, ...row.cells.map(cellView), ...lr.gotoRows[i].cells.map((c) => (c.target === null ? '' : String(c.target)))]),
    ),
  );
  if (lr.conflicts.length) {
    parts.push(h('h4', {}, t('tables.conflicts')), h('ul', {}, lr.conflicts.map((c) => h('li', { class: 'mono' }, t('tables.slr_conflict', { state: c.state, symbol: c.symbol, actions: c.actions.join(' / ') })))));
  }
  return parts;
}

registerPanel({
  id: 'grammar-tables',
  title: 'panel.grammar_tables',
  appliesTo: ['grammar'],
  order: 30,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    const output = h('div', { class: 'tables-output' });
    root.append(body);
    let token = 0;

    async function compute() {
      const mine = ++token;
      clear(output);
      if (!tab.doc.productions.length) {
        output.append(h('p', { class: 'muted' }, t('tables.empty')));
        return;
      }
      await guarded(tab, async () => {
        const result = await tab.api({ op: 'parse_tables', grammar: tab.doc });
        if (mine === token) clear(output).append(...tablesView(tablesModel(result)));
      });
    }

    body.append(h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: compute }, t('tables.compute'))), output);
    compute();
    const off = tab.on('doc', (e) => e.cause !== 'path' && e.cause !== 'flags' && compute());
    return { destroy: off, focus: compute };
  },
});

registerAction({ id: 'input.tables', menu: 'input', label: 'tables.compute', appliesTo: ['grammar'], order: 5, run: (tab) => tab.app.showPanel(tab, 'grammar-tables') });
