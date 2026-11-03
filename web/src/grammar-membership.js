// Grammar environment, membership by the sets Tk (CLASE 41): T0 = { S }, Tk = Tk-1 ∪ { β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β },
// repeated until Tk == Tk-1 or w ∈ Tk. Only for non-contracting grammars: the engine refuses the others.

import { h, clear, put } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { dataTable, statusLine, withLambda } from './widgets.js';
import { tkRows } from './tm-model.js';

registerStrings({
  en: {
    'panel.grammar_membership': 'Membership (Tk)',
    'tk.intro': 'Decides whether a word belongs to the language of a non-contracting grammar by building T0 = { S } and Tk = Tk-1 ∪ { β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β }.',
    'tk.input': 'Word to check (λ is the empty word)',
    'tk.max_forms': 'Limit of forms per set (optional)',
    'tk.run': 'Check',
    'tk.bound': 'N = |w| = {n}',
    'tk.col.set': 'Set',
    'tk.col.forms': 'Sentential forms',
    'tk.same': '{name} = {previous}: the sets stopped growing.',
    'tk.member': '"{word}" belongs to the language: it is in T{k}.',
    'tk.not_member': '"{word}" does not belong to the language: T{k} = T{previous}.',
    'tk.contracting': 'This grammar is contracting (some production makes the form shorter), and the Tk check only works for non-contracting grammars.',
    'tk.error': 'Error: {message}',
  },
  es: {
    'panel.grammar_membership': 'Pertenencia (Tk)',
    'tk.intro': 'Decide si una palabra pertenece al lenguaje de una gramática no compresora construyendo T0 = { S } y Tk = Tk-1 ∪ { β / |β| ≤ N ∧ α ∈ Tk-1 ∧ α ⇒ β }.',
    'tk.input': 'Palabra a comprobar (λ es la palabra vacía)',
    'tk.max_forms': 'Límite de formas por conjunto (opcional)',
    'tk.run': 'Comprobar',
    'tk.bound': 'N = |w| = {n}',
    'tk.col.set': 'Conjunto',
    'tk.col.forms': 'Formas sentenciales',
    'tk.same': '{name} = {previous}: los conjuntos dejaron de crecer.',
    'tk.member': '"{word}" pertenece al lenguaje: está en T{k}.',
    'tk.not_member': '"{word}" no pertenece al lenguaje: T{k} = T{previous}.',
    'tk.contracting': 'La gramática es compresora (alguna producción acorta la forma) y la comprobación por Tk solo vale para gramáticas no compresoras.',
    'tk.error': 'Error: {message}',
  },
});

registerPanel({
  id: 'grammar-membership',
  title: 'panel.grammar_membership',
  appliesTo: ['grammar'],
  order: 45,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    const word = h('input', { type: 'text', class: 'mono', spellcheck: 'false', autocomplete: 'off', 'aria-label': t('tk.input'), onkeydown: (e) => (e.key === 'Enter' ? check() : null) });
    const limit = h('input', { type: 'number', min: 1, class: 'narrow', 'aria-label': t('tk.max_forms') });
    const output = h('div', { class: 'tk-output', 'aria-live': 'polite' });
    body.append(
      h('p', { class: 'muted' }, t('tk.intro')),
      h('div', { class: 'form-row' }, h('label', {}, t('tk.input')), withLambda(word)),
      h('div', { class: 'form-row' }, h('label', {}, t('tk.max_forms')), limit),
      h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: () => check() }, t('tk.run'))),
      output,
    );
    root.append(body);

    async function check() {
      const text = word.value.trim() === 'λ' ? '' : word.value.trim();
      const request = { op: 'membership_tk', grammar: tab.doc, input: text };
      if (limit.value) request.max_forms = Number(limit.value);
      try {
        const r = await tab.api(request);
        const rows = tkRows(r);
        const last = rows[rows.length - 1];
        const shown = text === '' ? 'λ' : text;
        put(
          clear(output),
          h('p', { class: 'muted mono' }, t('tk.bound', { n: r.bound })),
          dataTable(t('panel.grammar_membership'), [t('tk.col.set'), t('tk.col.forms')], rows.map((row) => [h('span', { class: 'mono' }, row.name), h('span', { class: 'mono' }, row.text)]), { className: 'configs tk-table' }),
          last && last.same ? h('p', { class: 'muted' }, t('tk.same', { name: last.name, previous: `T${rows.length - 2}` })) : null,
          statusLine(r.member, t('tk.member', { word: shown, k: rows.length - 1 }), t('tk.not_member', { word: shown, k: rows.length - 1, previous: rows.length - 2 })),
        );
      } catch (error) {
        const message = error.code === 'contracting_grammar' ? `${t('tk.contracting')} (${error.message})` : error.message;
        clear(output).append(h('p', { class: 'form-error', role: 'alert' }, t('tk.error', { message })));
      }
    }

    return { destroy() {}, focus: () => word.focus(), check };
  },
});

registerAction({ id: 'input.membership_tk', menu: 'input', label: 'panel.grammar_membership', appliesTo: ['grammar'], order: 15, run: (tab) => tab.app.showPanel(tab, 'grammar-membership') });
