// Regular expression environment: the editor with live validation, the conversions (the step-by-step replay of
// RE -> NFA lives in the step viewer of the new tab), and equivalence against another tab.

import { h, clear, debounce, put } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { formDialog, messageDialog } from './dialogs.js';
import { withLambda } from './widgets.js';
import { constraintMarker } from './design-model.js';
import { convertToTab, guarded } from './actions.js';

registerStrings({
  en: {
    'panel.regex_editor': 'Expression',
    'file.new_regex': 'New regular expression',
    'regex.expression': 'Regular expression',
    'regex.valid': 'The expression is valid.',
    'regex.invalid': 'Not valid: {message}',
    'regex.apply': 'Use this expression',
    'regex.syntax': 'Syntax',
    'regex.syntax_text': '+ is union, * is star, writing symbols side by side is concatenation, parentheses group, λ is the empty string and ø the empty language. Example: (a+b)*abb.',
    'regex.convert': 'Convert',
    'regex.to_nfa': 'To NFA (step by step)',
    'regex.to_dfa': 'To DFA',
    'regex.to_min': 'To minimal DFA',
    'regex.convert_hint': 'The conversion opens a new tab; its "Steps" panel replays the de-expressionify steps with the generalized graph at each step.',
    'regex.equivalent': 'Compare with another tab…',
    'regex.equivalent_title': 'Are the two languages equal?',
    'regex.other_tab': 'Other document',
    'regex.need_other': 'Open another finite automaton, regular expression or regular grammar first.',
    'regex.are_equal': '"{a}" and "{b}" describe the same language.',
    'regex.not_equal': '"{a}" and "{b}" differ: the string {word} is accepted by only one of them.',
  },
  es: {
    'panel.regex_editor': 'Expresión',
    'file.new_regex': 'Nueva expresión regular',
    'regex.expression': 'Expresión regular',
    'regex.valid': 'La expresión es válida.',
    'regex.invalid': 'No es válida: {message}',
    'regex.apply': 'Usar esta expresión',
    'regex.syntax': 'Sintaxis',
    'regex.syntax_text': '+ es unión, * es clausura, escribir símbolos juntos es concatenación, los paréntesis agrupan, λ es la cadena vacía y ø el lenguaje vacío. Ejemplo: (a+b)*abb.',
    'regex.convert': 'Convertir',
    'regex.to_nfa': 'A AFN (paso a paso)',
    'regex.to_dfa': 'A AFD',
    'regex.to_min': 'A AFD mínimo',
    'regex.convert_hint': 'La conversión abre una pestaña nueva; su panel "Pasos" reproduce los pasos de "de-expressionify" con el grafo generalizado en cada paso.',
    'regex.equivalent': 'Comparar con otra pestaña…',
    'regex.equivalent_title': '¿Los dos lenguajes son iguales?',
    'regex.other_tab': 'Otro documento',
    'regex.need_other': 'Abrí primero otro autómata finito, expresión regular o gramática regular.',
    'regex.are_equal': '"{a}" y "{b}" describen el mismo lenguaje.',
    'regex.not_equal': '"{a}" y "{b}" difieren: la cadena {word} la acepta solo uno de los dos.',
  },
});

/** Pick one of the other open tabs that is a finite-automaton-like document, and compare languages. */
export async function askEquivalence(tab) {
  const others = tab.app.tabs().filter((x) => x !== tab && ['fa', 'regex', 'grammar'].includes(x.type));
  if (!others.length) {
    tab.notify(t('regex.need_other'), 'error');
    return;
  }
  const answer = await formDialog({
    title: t('regex.equivalent_title'),
    fields: [{ name: 'tab', label: t('regex.other_tab'), type: 'select', options: others.map((x) => [x.id, x.title]), value: others[0].id }],
  });
  if (!answer) return;
  const other = others.find((x) => String(x.id) === String(answer.tab));
  await guarded(tab, async () => {
    const r = await tab.api({ op: 'equivalent', a: tab.doc, b: other.doc });
    const text = r.equivalent ? t('regex.are_equal', { a: tab.title, b: other.title }) : t('regex.not_equal', { a: tab.title, b: other.title, word: r.counterexample === '' ? 'λ' : r.counterexample });
    await messageDialog({ title: t('regex.equivalent_title'), body: h('p', { role: 'status' }, text) });
  });
}

registerPanel({
  id: 'regex-editor',
  title: 'panel.regex_editor',
  appliesTo: ['regex'],
  order: 10,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    const input = h('input', { type: 'text', class: 'mono wide', id: `regex-${tab.id}`, value: tab.doc.expression || '', spellcheck: 'false', autocomplete: 'off', 'aria-describedby': `regex-status-${tab.id}` });
    const status = h('div', { class: 'regex-status', id: `regex-status-${tab.id}`, role: 'status', 'aria-live': 'polite' });
    let valid = false;
    let token = 0;

    const validate = debounce(async () => {
      const mine = ++token;
      const text = input.value;
      try {
        await tab.api({ op: 'normalize', document: { type: 'regex', expression: text } });
        if (mine !== token) return;
        valid = true;
        clear(status).append(h('p', { class: 'status-ok' }, h('span', { class: 'badge ok' }, '✓'), ` ${t('regex.valid')}`));
      } catch (error) {
        if (mine !== token) return;
        valid = false;
        const position = error.details && Number.isInteger(error.details.position) ? error.details.position : null;
        const marker = position === null ? null : constraintMarker(text, position);
        put(
          clear(status),
          h('p', { class: 'status-bad' }, h('span', { class: 'badge bad' }, '✗'), ` ${t('regex.invalid', { message: error.message })}`),
          marker ? h('p', { class: 'mono marker', 'aria-hidden': 'true' }, marker.before, h('mark', {}, marker.at || '␣'), marker.after) : null,
        );
      }
    }, 200);

    /** Commits the typed expression when it is valid and different; resolves with whether the tab holds it. */
    async function apply() {
      if (input.value === tab.doc.expression) return true;
      try {
        await tab.api({ op: 'normalize', document: { type: 'regex', expression: input.value } });
      } catch (error) {
        valid = false;
        tab.notify(t('regex.invalid', { message: error.message }), 'error');
        return false;
      }
      return tab.edit((d) => {
        d.expression = input.value;
      });
    }

    const convert = (to, label) => async () => {
      if (await apply()) convertToTab(tab, to, label);
    };

    input.addEventListener('input', validate);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        apply();
      }
    });
    body.append(
      h('div', { class: 'form-row' }, h('label', { for: input.id }, t('regex.expression')), withLambda(input)),
      status,
      h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: apply }, t('regex.apply'))),
      h('h3', {}, t('regex.syntax')),
      h('p', { class: 'muted' }, t('regex.syntax_text')),
      h('h3', {}, t('regex.convert')),
      h(
        'div',
        { class: 'button-row' },
        h('button', { type: 'button', onclick: convert('nfa', 'convert.nfa') }, t('regex.to_nfa')),
        h('button', { type: 'button', onclick: convert('dfa', 'convert.dfa') }, t('regex.to_dfa')),
        h('button', { type: 'button', onclick: convert('minimal_dfa', 'convert.minimal') }, t('regex.to_min')),
        h('button', { type: 'button', onclick: () => askEquivalence(tab) }, t('regex.equivalent')),
      ),
      h('p', { class: 'muted' }, t('regex.convert_hint')),
    );
    validate();
    const off = tab.on('doc', () => {
      if (document.activeElement !== input && input.value !== tab.doc.expression) {
        input.value = tab.doc.expression || '';
        validate();
      }
    });
    return { destroy: off, focus: () => input.focus() };
  },
});

registerAction({ id: 'file.new.regex', menu: 'file', label: 'file.new_regex', order: 5.1, run: (tab) => tab.app.newTab({ type: 'regex', expression: '' }) });
registerAction({ id: 'convert.equivalent.regex', menu: 'convert', label: 'regex.equivalent', appliesTo: ['regex'], order: 21, run: (tab) => askEquivalence(tab) });
