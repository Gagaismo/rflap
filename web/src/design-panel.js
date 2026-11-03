// Design panel (S4): describe a language (constraint, regular expression, regular grammar or a template for a
// non-regular one) with example strings; the engine builds, minimizes, verifies and explains the automaton.

import { h, clear } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { dataTable, svgImage } from './widgets.js';
import { DESIGN_MODES, DESIGN_TEMPLATES, DESIGN_MAX_CHECK_LEN, designDefaults, designRequest, designView, designErrorView } from './design-model.js';

registerStrings({
  en: {
    'panel.design': 'Design',
    'file.new_design': 'New automaton from a language…',
    'design.intro': 'Describe the language and give example strings: Rflap builds the minimal automaton, checks it against every string up to a length and tells you what each state means.',
    'design.alphabet': 'Alphabet',
    'design.alphabet_hint': 'The symbols of the strings, for example ab (spaces and commas are ignored).',
    'design.mode': 'The language is described by',
    'design.mode.constraint_text': 'A constraint',
    'design.mode.regex': 'A regular expression',
    'design.mode.regular_grammar': 'A regular grammar',
    'design.mode.template': 'A template (non-regular languages)',
    'design.text.constraint_text': 'Constraint',
    'design.text.regex': 'Regular expression',
    'design.text.regular_grammar': 'Regular grammar (one production per line)',
    'design.syntax_title': 'Constraint syntax',
    'design.syntax_help': 'Conditions: starts_with("w"), ends_with("w"), contains("w"), regex("r"), length <op> n, length % m == r, count(a) <op> n, count(a,b) % m == r, count(a) <op> count(b). Combine them with not, and, xor, or (tightest first) and parentheses. <op> is ==, !=, <, <=, > or >=. Example: ends_with("ab") and count(a) % 2 == 0.',
    'design.template': 'Template',
    'design.template.an_bn': 'aⁿbⁿ',
    'design.template.an_bm': 'aⁿbᵐ with a relation',
    'design.template.equal_count': 'As many a as b (any order)',
    'design.template.balanced_brackets': 'Balanced brackets',
    'design.template.wcwr': 'w c wᴿ',
    'design.template.even_palindrome': 'Even palindromes (w wᴿ)',
    'design.template.palindrome': 'All palindromes',
    'design.template.a': 'Symbol a',
    'design.template.b': 'Symbol b',
    'design.template.c': 'Separator c',
    'design.template.min': 'Minimum n (0 or 1)',
    'design.template.relation': 'Relation between n and m',
    'design.template.relation.le': 'n ≤ m',
    'design.template.relation.ge': 'n ≥ m',
    'design.template.relation.ne': 'n ≠ m',
    'design.template.pairs': 'Bracket pairs',
    'design.template.pairs_hint': 'Two characters per pair, separated by spaces: () []',
    'design.accept': 'Strings it must accept (one per line)',
    'design.reject': 'Strings it must reject (one per line)',
    'design.examples_hint': 'λ or "" is the empty string.',
    'design.options': 'Options',
    'design.check_len': 'Check every string up to this length',
    'design.trap': 'Keep the trap (sink) state',
    'design.run': 'Design the automaton',
    'design.running': 'Designing…',
    'design.error': 'The engine could not design it: {message}',
    'design.error_at': 'Where: ',
    'design.result': 'Result',
    'design.summary': '{kind} with {states} states. {transitions} transitions.',
    'design.kind.dfa': 'Minimal DFA',
    'design.kind.pda': 'Pushdown automaton',
    'design.kind.nfa': 'NFA',
    'design.verified': 'Verified: it agrees with the description on every example and every string checked.',
    'design.not_verified': 'Not verified: the automaton disagrees with some of your examples (in red below). Check the description or the examples.',
    'design.states': 'States and what they mean',
    'design.col.state': 'State',
    'design.col.role': 'Role',
    'design.col.meaning': 'Meaning',
    'design.col.reach': 'Reached by',
    'design.initial': 'initial',
    'design.accepting': 'accepting',
    'design.regex': 'Equivalent regular expression',
    'design.regex_none': 'No regular expression: the language is not regular, or the automaton is too large.',
    'design.verification': 'Verification',
    'design.examples': 'Your examples',
    'design.col.string': 'String',
    'design.col.expected': 'You expected',
    'design.col.got': 'Automaton says',
    'design.col.why': 'Note',
    'design.expected.accept': 'accept',
    'design.expected.reject': 'reject',
    'design.passed': 'ok',
    'design.failed': 'FAILED',
    'design.cross': 'Checked {n} strings up to length {len} against the description: {mismatches} disagreements.',
    'design.accepted': 'Shortest accepted strings',
    'design.open': 'Open the automaton in a new tab',
    'design.preview': 'Drawing of the designed automaton',
  },
  es: {
    'panel.design': 'Diseño',
    'file.new_design': 'Nuevo autómata desde un lenguaje…',
    'design.intro': 'Describí el lenguaje y dá cadenas de ejemplo: Rflap construye el autómata mínimo, lo comprueba contra todas las cadenas hasta una longitud y te dice qué significa cada estado.',
    'design.alphabet': 'Alfabeto',
    'design.alphabet_hint': 'Los símbolos de las cadenas, por ejemplo ab (se ignoran espacios y comas).',
    'design.mode': 'El lenguaje se describe con',
    'design.mode.constraint_text': 'Una restricción',
    'design.mode.regex': 'Una expresión regular',
    'design.mode.regular_grammar': 'Una gramática regular',
    'design.mode.template': 'Una plantilla (lenguajes no regulares)',
    'design.text.constraint_text': 'Restricción',
    'design.text.regex': 'Expresión regular',
    'design.text.regular_grammar': 'Gramática regular (una producción por línea)',
    'design.syntax_title': 'Sintaxis de las restricciones',
    'design.syntax_help': 'Condiciones: starts_with("w"), ends_with("w"), contains("w"), regex("r"), length <op> n, length % m == r, count(a) <op> n, count(a,b) % m == r, count(a) <op> count(b). Combinalas con not, and, xor, or (de la más fuerte a la más débil) y paréntesis. <op> es ==, !=, <, <=, > o >=. Ejemplo: ends_with("ab") and count(a) % 2 == 0.',
    'design.template': 'Plantilla',
    'design.template.an_bn': 'aⁿbⁿ',
    'design.template.an_bm': 'aⁿbᵐ con una relación',
    'design.template.equal_count': 'Tantas a como b (en cualquier orden)',
    'design.template.balanced_brackets': 'Paréntesis balanceados',
    'design.template.wcwr': 'w c wᴿ',
    'design.template.even_palindrome': 'Palíndromos pares (w wᴿ)',
    'design.template.palindrome': 'Todos los palíndromos',
    'design.template.a': 'Símbolo a',
    'design.template.b': 'Símbolo b',
    'design.template.c': 'Separador c',
    'design.template.min': 'Mínimo n (0 o 1)',
    'design.template.relation': 'Relación entre n y m',
    'design.template.relation.le': 'n ≤ m',
    'design.template.relation.ge': 'n ≥ m',
    'design.template.relation.ne': 'n ≠ m',
    'design.template.pairs': 'Pares de paréntesis',
    'design.template.pairs_hint': 'Dos caracteres por par, separados por espacios: () []',
    'design.accept': 'Cadenas que debe aceptar (una por línea)',
    'design.reject': 'Cadenas que debe rechazar (una por línea)',
    'design.examples_hint': 'λ o "" es la cadena vacía.',
    'design.options': 'Opciones',
    'design.check_len': 'Comprobar todas las cadenas hasta esta longitud',
    'design.trap': 'Conservar el estado trampa (sumidero)',
    'design.run': 'Diseñar el autómata',
    'design.running': 'Diseñando…',
    'design.error': 'El motor no pudo diseñarlo: {message}',
    'design.error_at': 'Dónde: ',
    'design.result': 'Resultado',
    'design.summary': '{kind} con {states} estados. {transitions} transiciones.',
    'design.kind.dfa': 'AFD mínimo',
    'design.kind.pda': 'Autómata de pila',
    'design.kind.nfa': 'AFN',
    'design.verified': 'Verificado: coincide con la descripción en todos los ejemplos y todas las cadenas comprobadas.',
    'design.not_verified': 'No verificado: el autómata no coincide con algunos de tus ejemplos (en rojo abajo). Revisá la descripción o los ejemplos.',
    'design.states': 'Estados y qué significan',
    'design.col.state': 'Estado',
    'design.col.role': 'Rol',
    'design.col.meaning': 'Significado',
    'design.col.reach': 'Se llega con',
    'design.initial': 'inicial',
    'design.accepting': 'de aceptación',
    'design.regex': 'Expresión regular equivalente',
    'design.regex_none': 'Sin expresión regular: el lenguaje no es regular o el autómata es demasiado grande.',
    'design.verification': 'Verificación',
    'design.examples': 'Tus ejemplos',
    'design.col.string': 'Cadena',
    'design.col.expected': 'Esperabas',
    'design.col.got': 'El autómata dice',
    'design.col.why': 'Nota',
    'design.expected.accept': 'aceptar',
    'design.expected.reject': 'rechazar',
    'design.passed': 'bien',
    'design.failed': 'FALLÓ',
    'design.cross': 'Se comprobaron {n} cadenas de hasta {len} símbolos contra la descripción: {mismatches} diferencias.',
    'design.accepted': 'Cadenas aceptadas más cortas',
    'design.open': 'Abrir el autómata en una pestaña nueva',
    'design.preview': 'Dibujo del autómata diseñado',
  },
});

const kindLabel = (kind) => (t(`design.kind.${kind}`) === `design.kind.${kind}` ? kind : t(`design.kind.${kind}`));

registerPanel({
  id: 'design',
  title: 'panel.design',
  appliesTo: ['*'],
  order: 80,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body design-panel' });
    root.append(body);
    const form = designDefaults();
    const id = (name) => `design-${tab.id}-${name}`;
    const output = h('div', { class: 'design-output', 'aria-live': 'polite' });
    const errorBox = h('div', { class: 'design-error', role: 'alert' });
    const templateBox = h('div', { class: 'design-template' });
    const textLabel = h('label', { for: id('text') });
    const text = h('textarea', { id: id('text'), rows: 3, class: 'mono', spellcheck: 'false', oninput: (e) => (form.text = e.target.value) });
    const syntax = h('details', { class: 'design-syntax' }, h('summary', {}, t('design.syntax_title')), h('p', { class: 'muted' }, t('design.syntax_help')));
    let last = null; // { result, view }
    let running = false;

    const bind = (name, control) => {
      control.addEventListener('input', () => (form[name] = control.type === 'checkbox' ? control.checked : control.value));
      return control;
    };
    const field = (name, label, control, hint) => h('div', { class: 'form-row' }, h('label', { for: control.id }, label), h('div', { class: 'form-input' }, control), hint ? h('small', {}, hint) : null);

    function drawMode() {
      textLabel.textContent = t(`design.text.${form.mode === 'template' ? 'constraint_text' : form.mode}`);
      const templated = form.mode === 'template';
      textLabel.parentElement.hidden = templated;
      syntax.hidden = form.mode !== 'constraint_text';
      templateBox.hidden = !templated;
    }

    function drawTemplate() {
      clear(templateBox);
      const symbol = (name) => bind(name, h('input', { type: 'text', id: id(name), value: form[name], class: 'narrow mono', maxlength: 1 }));
      const select = h('select', { id: id('template'), onchange: (e) => ((form.template = e.target.value), drawTemplate()) }, DESIGN_TEMPLATES.map((name) => h('option', { value: name, selected: name === form.template }, t(`design.template.${name}`))));
      templateBox.append(field('template', t('design.template'), select));
      if (['an_bn', 'an_bm', 'equal_count'].includes(form.template)) templateBox.append(field('a', t('design.template.a'), symbol('a')), field('b', t('design.template.b'), symbol('b')));
      if (form.template === 'an_bn') templateBox.append(field('min', t('design.template.min'), bind('min', h('input', { type: 'number', id: id('min'), min: 0, max: 1, value: form.min, class: 'narrow' }))));
      if (form.template === 'an_bm') {
        const relation = h('select', { id: id('relation'), onchange: (e) => (form.relation = e.target.value) }, ['le', 'ge', 'ne'].map((r) => h('option', { value: r, selected: r === form.relation }, t(`design.template.relation.${r}`))));
        templateBox.append(field('relation', t('design.template.relation'), relation));
      }
      if (form.template === 'wcwr') templateBox.append(field('c', t('design.template.c'), symbol('c')));
      if (form.template === 'balanced_brackets') templateBox.append(field('pairs', t('design.template.pairs'), bind('pairs', h('input', { type: 'text', id: id('pairs'), value: form.pairs, class: 'mono' })), t('design.template.pairs_hint')));
    }

    function showError(error, constraint) {
      clear(errorBox);
      if (!error) return;
      const view = error.code ? designErrorView(error, constraint) : { message: error.message, marker: null };
      errorBox.append(h('p', { class: 'form-error' }, error.code ? t('design.error', { message: view.message }) : view.message));
      if (view.marker) errorBox.append(h('p', { class: 'mono marker' }, t('design.error_at'), view.marker.before, h('mark', {}, view.marker.at || '␣'), view.marker.after));
    }

    async function run() {
      if (running) return;
      clear(errorBox);
      let request;
      try {
        request = designRequest(form);
      } catch (error) {
        showError(error);
        return;
      }
      running = true;
      clear(output).append(h('p', { class: 'muted', role: 'status' }, t('design.running')));
      try {
        const result = await tab.api(request);
        last = { result, view: designView(result) };
        clear(output);
        await drawResult();
      } catch (error) {
        clear(output);
        showError(error, form.text);
      } finally {
        running = false;
      }
    }

    async function drawResult() {
      const { result, view } = last;
      const open = () => tab.openTab(result.document, { title: `${kindLabel(view.kind)}: ${view.description || ''}`.slice(0, 60) });
      const picture = h('div', { class: 'design-preview' });
      output.append(
        h('h3', {}, t('design.result')),
        h('p', {}, t('design.summary', { kind: kindLabel(view.kind), states: view.stats.states, transitions: view.stats.transitions })),
        h('p', { class: view.ok ? 'status-ok' : 'status-bad', role: 'status' }, h('span', { class: view.ok ? 'badge ok' : 'badge bad' }, view.ok ? '✓' : '✗'), ' ', view.ok ? t('design.verified') : t('design.not_verified')),
        h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: open }, t('design.open'))),
        picture,
        h('h3', {}, t('design.states')),
        dataTable(
          t('design.states'),
          [t('design.col.state'), t('design.col.role'), t('design.col.meaning'), t('design.col.reach')],
          view.states.map((s) => [{ text: s.name, class: 'mono' }, [s.initial ? t('design.initial') : '', s.initial && s.accepting ? ', ' : '', s.accepting ? t('design.accepting') : ''].join(''), s.meaning, { text: s.accessWord === null || s.accessWord === undefined ? '' : s.accessWord === '' ? 'λ' : s.accessWord, class: 'mono' }]),
        ),
        h('h3', {}, t('design.regex')),
        view.regex ? h('pre', { class: 'description mono' }, view.regex) : h('p', { class: 'muted' }, t('design.regex_none')),
        h('h3', {}, t('design.verification')),
      );
      if (view.examples.length) {
        output.append(
          h('h4', {}, t('design.examples')),
          dataTable(
            t('design.examples'),
            [t('design.col.string'), t('design.col.expected'), t('design.col.got'), t('design.col.why')],
            view.examples.map((e) => [
              { text: e.word === '' ? 'λ' : e.word, class: 'mono' },
              e.expected ? t('design.expected.accept') : t('design.expected.reject'),
              { text: e.actual ? t('design.expected.accept') : t('design.expected.reject'), class: e.passed ? 'outcome-accepted' : 'outcome-rejected' },
              e.passed ? t('design.passed') : [t('design.failed'), e.detail ? ` — ${e.detail}` : ''].join(''),
            ]),
            { rowClass: (i) => (view.examples[i].passed ? '' : 'failed-row') },
          ),
        );
      }
      output.append(h('p', { class: view.crossCheck.mismatches ? 'status-bad' : 'muted' }, t('design.cross', { n: view.crossCheck.checked, len: view.crossCheck.maxLen, mismatches: view.crossCheck.mismatches })));
      if (view.accepted.length) output.append(h('h4', {}, t('design.accepted')), h('p', { class: 'mono' }, view.accepted.map((w) => (w === '' ? 'λ' : w)).join('   ')));
      try {
        const drawn = await tab.api({ op: 'render', document: result.document, format: 'svg', dark: Boolean(tab.dark) });
        picture.append(svgImage(drawn.content, t('design.preview')));
      } catch {
        /* the report stands on its own */
      }
    }

    const alphabet = bind('alphabet', h('input', { type: 'text', id: id('alphabet'), value: form.alphabet, class: 'mono', spellcheck: 'false', autocomplete: 'off' }));
    const mode = h('select', { id: id('mode'), onchange: (e) => ((form.mode = e.target.value), drawMode()) }, DESIGN_MODES.map((m) => h('option', { value: m, selected: m === form.mode }, t(`design.mode.${m}`))));
    const accept = bind('accept', h('textarea', { id: id('accept'), rows: 3, class: 'mono', spellcheck: 'false' }));
    const reject = bind('reject', h('textarea', { id: id('reject'), rows: 3, class: 'mono', spellcheck: 'false' }));
    const checkLen = bind('checkLen', h('input', { type: 'number', id: id('check'), min: 0, max: DESIGN_MAX_CHECK_LEN, value: form.checkLen, class: 'narrow' }));
    const trap = bind('trap', h('input', { type: 'checkbox', id: id('trap'), checked: form.trap }));

    body.append(
      h('p', { class: 'muted' }, t('design.intro')),
      field('alphabet', t('design.alphabet'), alphabet, t('design.alphabet_hint')),
      field('mode', t('design.mode'), mode),
      h('div', { class: 'form-row' }, textLabel, h('div', { class: 'form-input' }, text)),
      syntax,
      templateBox,
      field('accept', t('design.accept'), accept, t('design.examples_hint')),
      field('reject', t('design.reject'), reject),
      h('details', {}, h('summary', {}, t('design.options')), field('check', t('design.check_len'), checkLen), h('div', { class: 'form-row' }, h('label', { for: id('trap') }, t('design.trap')), h('div', { class: 'form-input' }, trap))),
      errorBox,
      h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: run }, t('design.run'))),
      output,
    );
    drawTemplate();
    drawMode();
    return { destroy() {}, focus: () => alphabet.focus() };
  },
});

registerAction({
  id: 'file.new.design',
  menu: 'file',
  label: 'file.new_design',
  order: 5.9,
  run: (tab) => tab.app.showPanel(tab, 'design'),
});
