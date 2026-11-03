// The Turing machine and ALL environment in the notation of the course "Sintaxis y Semántica de los Lenguajes"
// (CLASE 39 and CLASE 41): formal definition, transition table, configuration sequences, the ALL check, the
// composition of machines, the cátedra / JFLAP notation switch and the File > New entries for MT and ALL.
// Everything goes through `ctx.api` (the ops `catedra_definition`, `catedra_table`, `catedra_run`, `all_check`,
// `compose_tm`); the pure parts (view models) are in tm-model.js.

import { h, clear, debounce, put } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { container, emptyDocument } from './doc.js';
import { dataTable } from './widgets.js';
import { formDialog } from './dialogs.js';
import { guarded, copyText } from './actions.js';
import { CONFIG_STEP, definitionRows, effectiveInputs, sequenceView, statesOfRun, violationTransitions } from './tm-model.js';

registerStrings({
  en: {
    'file.new_all': 'Linear bounded automaton (ALL)',
    'panel.catedra_definition': 'Formal definition',
    'panel.catedra_table': 'Transition table',
    'panel.catedra_run': 'Configuration sequence',
    'panel.catedra_all': 'ALL check',
    'view.notation_catedra': 'Notation: course (e,s,m)',
    'view.notation_jflap': 'Notation: JFLAP (e;s,m)',
    'input.catedra_run': 'Configuration sequence (├─)',
    'test.all_check': 'Check ALL',
    'convert.compose_tm': 'Compose with another Turing machine…',
    'convert.compose_title': 'Compose with another Turing machine',
    'convert.composed_tm': '{a} ∘ {b}',

    'catedra.deterministic': 'Deterministic',
    'catedra.sigma_inferred': 'Σ_E was inferred: the input symbols are the ones the machine reads that are not auxiliary. Declare it to fix it.',
    'catedra.sigma_declared': 'Σ_E is declared in the document.',
    'catedra.sigma_edit': 'Σ_E (symbols, for example ab)',
    'catedra.sigma_apply': 'Declare Σ_E',
    'catedra.sigma_infer': 'Infer Σ_E again',
    'catedra.issues': 'Problems found',
    'catedra.no_issues': 'No problems: the machine fits the definition of the course.',
    'catedra.highlight': 'Highlight',
    'catedra.table_copy': 'Copy as Markdown',
    'catedra.table_hint': '→ marks the initial state, * the final states (halting: they have no transitions).',
    'catedra.model': 'Run as',
    'catedra.model_mt': 'MT (Turing machine)',
    'catedra.model_all': 'ALL (linear bounded automaton)',
    'catedra.inputs': 'Input words (λ or an empty line is the empty word)',
    'catedra.add_input': 'Add input',
    'catedra.remove_input': 'Remove input {n}',
    'catedra.input_n': 'Input {n}',
    'catedra.max_steps': 'Step limit',
    'catedra.run': 'Run',
    'catedra.word': 'Word: {word}',
    'catedra.empty_word': 'λ',
    'catedra.steps': '{n} steps',
    'catedra.step_prev': 'Previous configuration',
    'catedra.step_next': 'Next configuration',
    'catedra.step_of': 'Configuration {n} of {total}',
    'catedra.violation': 'ALL violation: {message}',
    'catedra.run_hint': 'Δ is the blank. The head stands right after the state written in the configuration.',
    'catedra.verdict.accepted': 'accepts',
    'catedra.verdict.rejected': 'rejects',
    'catedra.verdict.no_halt': 'does not halt',
    'catedra.verdict.violation': 'violates the ALL rules',
    'catedra.all_run': 'Check',
    'catedra.all_ok': 'The machine follows the rules of an ALL.',
    'catedra.all_bad': 'The machine breaks the rules of an ALL.',
    'catedra.all_static': 'Rules of the transitions',
    'catedra.all_dynamic': 'While running',
    'catedra.all_none': 'No violations.',
    'catedra.all_inputs': 'The check runs the words of the "Configuration sequence" panel.',
    'catedra.all_no_inputs': 'No input words: only the transitions are checked. Write words in "Configuration sequence" to also check the runs.',
    'catedra.all_transition': 'Transition {n}',
    'catedra.all_step': 'step {n}, word {word}',
    'catedra.pick_other': 'Other Turing machine',
    'catedra.need_other': 'Open another Turing machine in a tab first.',
    'catedra.error': 'Error: {message}',
  },
  es: {
    'file.new_all': 'Autómata Linealmente Limitado (ALL)',
    'panel.catedra_definition': 'Definición formal',
    'panel.catedra_table': 'Tabla de transición',
    'panel.catedra_run': 'Secuencia de configuración',
    'panel.catedra_all': 'Verificación ALL',
    'view.notation_catedra': 'Notación: cátedra (e,s,m)',
    'view.notation_jflap': 'Notación: JFLAP (e;s,m)',
    'input.catedra_run': 'Secuencia de configuración (├─)',
    'test.all_check': 'Verificar ALL',
    'convert.compose_tm': 'Componer con otra MT…',
    'convert.compose_title': 'Componer con otra máquina de Turing',
    'convert.composed_tm': '{a} ∘ {b}',

    'catedra.deterministic': 'Determinista',
    'catedra.sigma_inferred': 'Σ_E se infirió: son los símbolos que la máquina lee y que no son auxiliares. Declaralo para fijarlo.',
    'catedra.sigma_declared': 'Σ_E está declarado en el documento.',
    'catedra.sigma_edit': 'Σ_E (símbolos, por ejemplo ab)',
    'catedra.sigma_apply': 'Declarar Σ_E',
    'catedra.sigma_infer': 'Volver a inferir Σ_E',
    'catedra.issues': 'Problemas encontrados',
    'catedra.no_issues': 'Sin problemas: la máquina cumple la definición de la cátedra.',
    'catedra.highlight': 'Resaltar',
    'catedra.table_copy': 'Copiar como Markdown',
    'catedra.table_hint': '→ marca el estado inicial y * los finales (de parada: no tienen transiciones).',
    'catedra.model': 'Ejecutar como',
    'catedra.model_mt': 'MT (máquina de Turing)',
    'catedra.model_all': 'ALL (autómata linealmente limitado)',
    'catedra.inputs': 'Palabras de entrada (λ o una línea vacía es la palabra vacía)',
    'catedra.add_input': 'Agregar entrada',
    'catedra.remove_input': 'Quitar la entrada {n}',
    'catedra.input_n': 'Entrada {n}',
    'catedra.max_steps': 'Límite de pasos',
    'catedra.run': 'Ejecutar',
    'catedra.word': 'Palabra: {word}',
    'catedra.empty_word': 'λ',
    'catedra.steps': '{n} pasos',
    'catedra.step_prev': 'Configuración anterior',
    'catedra.step_next': 'Configuración siguiente',
    'catedra.step_of': 'Configuración {n} de {total}',
    'catedra.violation': 'Violación del ALL: {message}',
    'catedra.run_hint': 'Δ es el blanco. El cabezal está justo después del estado que figura en la configuración.',
    'catedra.verdict.accepted': 'acepta',
    'catedra.verdict.rejected': 'rechaza',
    'catedra.verdict.no_halt': 'no se detiene',
    'catedra.verdict.violation': 'viola las reglas del ALL',
    'catedra.all_run': 'Verificar',
    'catedra.all_ok': 'La máquina cumple las reglas de un ALL.',
    'catedra.all_bad': 'La máquina rompe las reglas de un ALL.',
    'catedra.all_static': 'Reglas de las transiciones',
    'catedra.all_dynamic': 'Al ejecutar',
    'catedra.all_none': 'Sin violaciones.',
    'catedra.all_inputs': 'La verificación ejecuta las palabras del panel "Secuencia de configuración".',
    'catedra.all_no_inputs': 'No hay palabras de entrada: solo se revisan las transiciones. Escribí palabras en "Secuencia de configuración" para revisar también las ejecuciones.',
    'catedra.all_transition': 'Transición {n}',
    'catedra.all_step': 'paso {n}, palabra {word}',
    'catedra.pick_other': 'Otra máquina de Turing',
    'catedra.need_other': 'Abrí primero otra máquina de Turing en una pestaña.',
    'catedra.error': 'Error: {message}',
  },
});

const TURING = ['turing'];
const OVERLAY = 'catedra';

const errorLine = (error) => h('p', { class: 'form-error', role: 'alert' }, error.message);
const wordText = (word) => (word === '' ? t('catedra.empty_word') : word);

/** The run settings shared by the sequence panel and the ALL check: `tab.catedra = { rows, maxSteps }`. */
export function catedraSettings(tab) {
  if (!tab.catedra) tab.catedra = { rows: [''], maxSteps: 1000 };
  return tab.catedra;
}

const stateNames = (tab) => Object.values(container(tab.doc).states);
const idOfName = (tab, name) => {
  const found = stateNames(tab).find((s) => s.name === name);
  return found ? found.id : null;
};

/** Redraws `draw` after the document, the model or the language change, without piling up requests. */
function refreshOn(tab, draw, events = ['doc', 'model']) {
  const redraw = debounce(draw, 80);
  const offs = events.map((event) => tab.on(event, (e) => (e && e.cause === 'flags' ? undefined : redraw())));
  return () => offs.forEach((off) => off());
}

// ------------------------------------------------------------------------------------ formal definition

registerPanel({
  id: 'catedra-definition',
  title: 'panel.catedra_definition',
  appliesTo: TURING,
  order: 32,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    let seq = 0;

    async function draw() {
      const mine = ++seq;
      let definition;
      try {
        definition = await tab.api({ op: 'catedra_definition', document: tab.doc, model: tab.model });
      } catch (error) {
        if (mine === seq) clear(body).append(errorLine(error));
        return;
      }
      if (mine !== seq) return;
      const view = definitionRows(definition);
      const sigma = h('input', { type: 'text', class: 'mono narrow', value: definition.input_alphabet_inferred ? '' : definition.input_alphabet.join(''), spellcheck: 'false', autocomplete: 'off', 'aria-label': t('catedra.sigma_edit'), placeholder: definition.input_alphabet.join('') });
      const apply = (symbols) =>
        tab.edit((d) => {
          if (symbols === null) delete d.input_alphabet;
          else d.input_alphabet = [...new Set([...symbols.replace(/[\s,]/g, '')])];
        });
      tab.setOverlay(OVERLAY, null);
      clear(body).append(
        h('h3', { class: 'formal' }, view.header),
        h('dl', { class: 'facts' }, view.rows.map((r) => [h('dt', { class: 'mono' }, r.symbol), h('dd', { class: 'mono' }, r.set)])),
        h('p', { class: 'muted' }, definition.input_alphabet_inferred ? t('catedra.sigma_inferred') : t('catedra.sigma_declared')),
        h(
          'div',
          { class: 'button-row' },
          h('label', { class: 'inline' }, t('catedra.sigma_edit'), sigma),
          h('button', { type: 'button', onclick: () => apply(sigma.value) }, t('catedra.sigma_apply')),
          h('button', { type: 'button', disabled: definition.input_alphabet_inferred, onclick: () => apply(null) }, t('catedra.sigma_infer')),
        ),
        h('p', {}, `${t('catedra.deterministic')}: ${definition.deterministic ? t('word.yes') : t('word.no')}`),
        h('h3', {}, t('catedra.issues')),
        definition.issues.length
          ? h(
              'ul',
              { class: 'issues' },
              definition.issues.map((issue) =>
                h(
                  'li',
                  {},
                  issue.message,
                  issue.transitions && issue.transitions.length
                    ? h('button', { type: 'button', onclick: () => tab.setOverlay(OVERLAY, { transitions: issue.transitions }) }, t('catedra.highlight'))
                    : null,
                ),
              ),
            )
          : h('p', { class: 'status-ok', role: 'status' }, t('catedra.no_issues')),
      );
    }

    draw();
    const off = refreshOn(tab, draw);
    return { destroy: off, refresh: draw };
  },
});

// ------------------------------------------------------------------------------------ transition table

registerPanel({
  id: 'catedra-table',
  title: 'panel.catedra_table',
  appliesTo: TURING,
  order: 34,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    let seq = 0;

    async function draw() {
      const mine = ++seq;
      let table;
      try {
        table = await tab.api({ op: 'catedra_table', document: tab.doc });
      } catch (error) {
        if (mine === seq) clear(body).append(errorLine(error));
        return;
      }
      if (mine !== seq) return;
      clear(body).append(
        dataTable(t('panel.catedra_table'), table.header, table.rows, { className: 'configs catedra-table' }),
        h('p', { class: 'muted' }, t('catedra.table_hint')),
        h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => copyText(table.markdown, t('catedra.table_copy')) }, t('catedra.table_copy'))),
      );
    }

    draw();
    const off = refreshOn(tab, draw, ['doc']);
    return { destroy: off, refresh: draw };
  },
});

// ------------------------------------------------------------------------------------ configuration sequence

registerPanel({
  id: 'catedra-run',
  title: 'panel.catedra_run',
  appliesTo: TURING,
  order: 36,
  mount(root, tab) {
    const settings = catedraSettings(tab);
    const body = h('div', { class: 'panel-body' });
    const rowsBox = h('div', { class: 'catedra-rows' });
    const results = h('div', { class: 'catedra-results', 'aria-live': 'polite' });
    const modelSelect = h(
      'select',
      { 'aria-label': t('catedra.model'), onchange: (e) => tab.setModel(e.target.value) },
      ['mt', 'all'].map((m) => h('option', { value: m, selected: tab.model === m }, t(`catedra.model_${m}`))),
    );
    const maxSteps = h('input', { type: 'number', min: 1, max: 100000, class: 'narrow', value: String(settings.maxSteps), 'aria-label': t('catedra.max_steps'), oninput: (e) => (settings.maxSteps = Number(e.target.value) || 1000) });
    body.append(
      h('p', { class: 'muted' }, t('catedra.run_hint')),
      rowsBox,
      h('div', { class: 'button-row' }, h('button', { type: 'button', onclick: () => ((settings.rows.push(''), drawRows(), focusLast())) }, t('catedra.add_input')), h('button', { type: 'button', class: 'primary', onclick: () => run() }, t('catedra.run'))),
      h('div', { class: 'button-row' }, h('label', { class: 'inline' }, t('catedra.model'), modelSelect), h('label', { class: 'inline' }, t('catedra.max_steps'), maxSteps)),
      results,
    );
    root.append(body);
    function focusLast() {
      const inputs = rowsBox.querySelectorAll('input');
      if (inputs.length) inputs[inputs.length - 1].focus();
    }

    function drawRows() {
      clear(rowsBox);
      settings.rows.forEach((value, i) => {
        rowsBox.append(
          h(
            'div',
            { class: 'catedra-row' },
            h('input', {
              type: 'text',
              class: 'mono',
              value,
              spellcheck: 'false',
              autocomplete: 'off',
              'aria-label': t('catedra.input_n', { n: i + 1 }),
              placeholder: t('catedra.input_n', { n: i + 1 }),
              oninput: (e) => (settings.rows[i] = e.target.value),
              onkeydown: (e) => (e.key === 'Enter' ? run() : null),
            }),
            settings.rows.length > 1
              ? h('button', { type: 'button', 'aria-label': t('catedra.remove_input', { n: i + 1 }), onclick: () => ((settings.rows.splice(i, 1), drawRows())) }, '✕')
              : null,
          ),
        );
      });
    }

    function showStep(names, step) {
      const state = names[step];
      tab.setOverlay(OVERLAY, state == null ? null : { states: [idOfName(tab, state)].filter((x) => x != null) });
    }

    function resultView(entry) {
      const view = sequenceView(entry);
      const initial = (container(tab.doc).initial != null && container(tab.doc).states[container(tab.doc).initial]) || null;
      const names = statesOfRun(view.configs, stateNames(tab).map((s) => s.name).sort((a, b) => b.length - a.length), initial ? initial.name : null);
      let step = view.configs.length - 1;
      const sequence = h('p', { class: 'sequence mono', tabindex: '0' });
      const position = h('span', { class: 'muted', role: 'status' });
      const prev = h('button', { type: 'button', 'aria-label': t('catedra.step_prev'), onclick: () => go(step - 1) }, '◀');
      const next = h('button', { type: 'button', 'aria-label': t('catedra.step_next'), onclick: () => go(step + 1) }, '▶');

      function paint() {
        clear(sequence);
        view.configs.forEach((config, i) => {
          if (i) sequence.append(` ${CONFIG_STEP} `);
          sequence.append(h('button', { type: 'button', class: i === step ? 'cfg current' : 'cfg', 'aria-current': i === step ? 'step' : null, onclick: () => go(i) }, config));
        });
        if (view.tail) sequence.append(' ', h('span', { class: `seq-tail seq-${view.verdictClass}` }, view.tail));
        position.textContent = t('catedra.step_of', { n: step + 1, total: view.configs.length });
        prev.disabled = step <= 0;
        next.disabled = step >= view.configs.length - 1;
      }

      function go(to) {
        step = Math.max(0, Math.min(view.configs.length - 1, to));
        paint();
        showStep(names, step);
      }

      paint();
      return {
        node: h(
          'section',
          { class: `catedra-result result-${view.verdictClass}`, 'aria-label': t('catedra.word', { word: wordText(view.input) }) },
          h('h4', {}, t('catedra.word', { word: wordText(view.input) }), ' ', h('span', { class: `badge verdict-${view.verdictClass}` }, t(`catedra.verdict.${view.verdict}`) ), ' ', h('span', { class: 'muted' }, t('catedra.steps', { n: view.steps }))),
          sequence,
          view.violation ? h('p', { class: 'warn' }, t('catedra.violation', { message: view.violation.message })) : null,
          h('div', { class: 'button-row' }, prev, next, position),
        ),
        go,
        last: () => go(view.configs.length - 1),
      };
    }

    async function run() {
      const inputs = effectiveInputs(settings.rows, { fallback: true });
      tab.setOverlay(OVERLAY, null);
      try {
        const r = await tab.api({ op: 'catedra_run', document: tab.doc, inputs, model: tab.model, max_steps: settings.maxSteps });
        clear(results);
        const views = r.results.map(resultView);
        views.forEach((v) => results.append(v.node));
        if (views.length === 1) views[0].last();
      } catch (error) {
        clear(results).append(errorLine(error));
      }
    }

    drawRows();
    const offs = [
      tab.on('doc', (e) => {
        if (e.cause === 'path' || e.cause === 'flags') return;
        clear(results);
        tab.setOverlay(OVERLAY, null);
      }),
      tab.on('model', () => {
        modelSelect.value = tab.model;
        clear(results);
      }),
    ];
    return {
      destroy: () => offs.forEach((off) => off()),
      focus: () => rowsBox.querySelector('input')?.focus(),
      run,
    };
  },
});

// ------------------------------------------------------------------------------------ ALL check

registerPanel({
  id: 'catedra-all',
  title: 'panel.catedra_all',
  appliesTo: TURING,
  order: 38,
  mount(root, tab) {
    const settings = catedraSettings(tab);
    const body = h('div', { class: 'panel-body' });
    const output = h('div', { class: 'catedra-all', 'aria-live': 'polite' });
    body.append(h('p', { class: 'muted' }, t('catedra.all_inputs')), h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: () => check() }, t('catedra.all_run'))), output);
    root.append(body);

    function violationItem(v) {
      const where = v.input !== undefined ? t('catedra.all_step', { n: v.step, word: wordText(v.input) }) : t('catedra.all_transition', { n: v.transition + 1 });
      return h(
        'li',
        {},
        h('span', { class: 'muted' }, `${where}: `),
        v.message,
        Number.isInteger(v.transition) ? h('button', { type: 'button', onclick: () => tab.setOverlay(OVERLAY, { transitions: [v.transition] }) }, t('catedra.highlight')) : null,
      );
    }

    async function check() {
      const inputs = effectiveInputs(settings.rows);
      try {
        const r = await tab.api({ op: 'all_check', document: tab.doc, inputs, max_steps: settings.maxSteps });
        const staticOnes = r.violations.filter((v) => v.input === undefined);
        const dynamicOnes = r.violations.filter((v) => v.input !== undefined);
        tab.setOverlay(OVERLAY, { transitions: violationTransitions(r.violations) });
        put(
          clear(output),
          h('p', { class: r.ok ? 'status-ok' : 'status-bad', role: 'status' }, h('span', { class: r.ok ? 'badge ok' : 'badge bad' }, r.ok ? '✓' : '✗'), ' ', r.ok ? t('catedra.all_ok') : t('catedra.all_bad')),
          inputs.length ? null : h('p', { class: 'muted' }, t('catedra.all_no_inputs')),
          h('h3', {}, t('catedra.all_static')),
          staticOnes.length ? h('ul', { class: 'issues' }, staticOnes.map(violationItem)) : h('p', { class: 'muted' }, t('catedra.all_none')),
          inputs.length ? [h('h3', {}, t('catedra.all_dynamic')), dynamicOnes.length ? h('ul', { class: 'issues' }, dynamicOnes.map(violationItem)) : h('p', { class: 'muted' }, t('catedra.all_none'))] : null,
        );
      } catch (error) {
        clear(output).append(errorLine(error));
      }
    }

    const off = tab.on('doc', (e) => {
      if (e.cause === 'path' || e.cause === 'flags') return;
      clear(output);
      tab.setOverlay(OVERLAY, null);
    });
    return { destroy: off, check, focus: () => body.querySelector('button')?.focus() };
  },
});

// ------------------------------------------------------------------------------------ menu actions

registerAction({
  id: 'file.new.all',
  menu: 'file',
  label: 'file.new_all',
  order: 2.5,
  run: (tab) => tab.app.newTab(emptyDocument('turing', 1), { model: 'all' }),
});

registerAction({ id: 'input.catedra_run', menu: 'input', label: 'input.catedra_run', order: 5, appliesTo: TURING, run: (tab) => tab.app.showPanel(tab, 'catedra-run') });

registerAction({
  id: 'test.all_check',
  menu: 'test',
  label: 'test.all_check',
  order: 3,
  appliesTo: TURING,
  run: (tab) => {
    tab.app.showPanel(tab, 'catedra-all', 'check');
  },
});

registerAction({
  id: 'convert.compose_tm',
  menu: 'convert',
  label: 'convert.compose_tm',
  order: 30,
  separator: true,
  appliesTo: TURING,
  run: async (tab) => {
    const others = tab.app.tabs().filter((x) => x !== tab && x.type === 'turing');
    if (!others.length) {
      tab.notify(t('catedra.need_other'), 'error');
      return;
    }
    const answer = await formDialog({
      title: t('convert.compose_title'),
      fields: [{ name: 'tab', label: t('catedra.pick_other'), type: 'select', options: others.map((x) => [x.id, x.title]), value: others[0].id }],
    });
    if (!answer) return;
    const other = others.find((x) => String(x.id) === String(answer.tab));
    guarded(tab, async () => {
      const r = await tab.api({ op: 'compose_tm', a: tab.doc, b: other.doc });
      tab.openTab(r.document, { title: t('convert.composed_tm', { a: tab.title, b: other.title }) });
    });
  },
});

for (const [index, notation] of ['catedra', 'jflap'].entries()) {
  registerAction({
    id: `view.notation.${notation}`,
    menu: 'view',
    label: `view.notation_${notation}`,
    order: 10 + index,
    separator: index === 0,
    appliesTo: TURING,
    checked: (tab) => tab.notation === notation,
    run: (tab) => tab.app.setNotation(notation),
  });
}
