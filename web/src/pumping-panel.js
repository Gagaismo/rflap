// Pumping lemma environment: the game of JFLAP in its box order (m, w, decomposition, i, result), for the regular
// and the context-free lemma, with the cases panel of the context-free "you go first" game. The engine plays
// the computer (`pumping_new` / `pumping_move`); the tab's document is the game state, so undo steps back a move.

import { h, clear } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { richSpan } from './richtext.js';
import { guarded } from './actions.js';
import { pumpingView, decompositionParts, clampLengths, indicesFromLengths, lengthsFromIndices, pumpingLanguages, currentLanguage } from './pumping-model.js';

registerStrings({
  en: {
    'panel.pumping': 'Pumping lemma',
    'file.new_pumping_regular': 'New pumping lemma (regular)',
    'file.new_pumping_cf': 'New pumping lemma (context-free)',
    'pumping.setup': 'Game',
    'pumping.language': 'Language',
    'pumping.mode': 'Who goes first',
    'pumping.mode.computer_first': 'The computer goes first (you try to prevent a valid partition)',
    'pumping.mode.human_first': 'You go first (you try to find a partition that can be pumped)',
    'pumping.new_game': 'New game',
    'pumping.clear_all': 'Clear all',
    'pumping.objective': 'Objective',
    'pumping.box': 'Box {n}',
    'pumping.enter': 'Enter',
    'pumping.m': 'm',
    'pumping.w': 'w',
    'pumping.w_hint': 'The string w must have at least m symbols and be in the language.',
    'pumping.i': 'i',
    'pumping.decompose': 'Decompose',
    'pumping.decomposition_of': 'Decomposition of w',
    'pumping.length_of': 'Length of {part}',
    'pumping.end_of': 'Where {part} ends (index in w)',
    'pumping.pumped': 'Pumped string',
    'pumping.attempts': 'Attempts',
    'pumping.win_user': 'You win.',
    'pumping.win_computer': 'The computer wins this round.',
    'pumping.conclusion.not_regular': 'Conclusion: the language is not regular.',
    'pumping.conclusion.not_context_free': 'Conclusion: the language is not context-free.',
    'pumping.explanation': 'Explanation of the language (shows the answer)',
    'pumping.cases': 'Cases',
    'pumping.cases_total': '{n} cases to cover.',
    'pumping.cases_none': 'No cases yet.',
    'pumping.case_add': 'Add case',
    'pumping.case_replace': 'Replace case',
    'pumping.case_show': 'Show case',
    'pumping.case_delete': 'Delete case',
    'pumping.case_list': 'List all cases',
    'pumping.case_clear': 'Clear cases',
    'pumping.case_done': 'Done',
    'pumping.case_select': 'Select a case first.',
    'pumping.reply_rejected': 'Not accepted: {message}',
  },
  es: {
    'panel.pumping': 'Lema de bombeo',
    'file.new_pumping_regular': 'Nuevo lema de bombeo (regular)',
    'file.new_pumping_cf': 'Nuevo lema de bombeo (libre de contexto)',
    'pumping.setup': 'Juego',
    'pumping.language': 'Lenguaje',
    'pumping.mode': 'Quién empieza',
    'pumping.mode.computer_first': 'Empieza la computadora (vos tratás de impedir una partición válida)',
    'pumping.mode.human_first': 'Empezás vos (tratás de encontrar una partición que se pueda bombear)',
    'pumping.new_game': 'Juego nuevo',
    'pumping.clear_all': 'Borrar todo',
    'pumping.objective': 'Objetivo',
    'pumping.box': 'Caja {n}',
    'pumping.enter': 'Enter',
    'pumping.m': 'm',
    'pumping.w': 'w',
    'pumping.w_hint': 'La cadena w debe tener al menos m símbolos y estar en el lenguaje.',
    'pumping.i': 'i',
    'pumping.decompose': 'Descomponer',
    'pumping.decomposition_of': 'Descomposición de w',
    'pumping.length_of': 'Largo de {part}',
    'pumping.end_of': 'Dónde termina {part} (índice en w)',
    'pumping.pumped': 'Cadena bombeada',
    'pumping.attempts': 'Intentos',
    'pumping.win_user': 'Ganaste.',
    'pumping.win_computer': 'Gana la computadora esta ronda.',
    'pumping.conclusion.not_regular': 'Conclusión: el lenguaje no es regular.',
    'pumping.conclusion.not_context_free': 'Conclusión: el lenguaje no es libre de contexto.',
    'pumping.explanation': 'Explicación del lenguaje (muestra la respuesta)',
    'pumping.cases': 'Casos',
    'pumping.cases_total': 'Hay {n} casos por cubrir.',
    'pumping.cases_none': 'Todavía no hay casos.',
    'pumping.case_add': 'Agregar caso',
    'pumping.case_replace': 'Reemplazar caso',
    'pumping.case_show': 'Mostrar caso',
    'pumping.case_delete': 'Borrar caso',
    'pumping.case_list': 'Listar todos los casos',
    'pumping.case_clear': 'Borrar los casos',
    'pumping.case_done': 'Listo',
    'pumping.case_select': 'Elegí primero un caso.',
    'pumping.reply_rejected': 'No se aceptó: {message}',
  },
});

const KINDS = { regular_pumping_lemma: 'regular', context_free_pumping_lemma: 'context_free' };
const PART_NAMES = { 2: ['x', 'y', 'z'], 4: ['u', 'v', 'x', 'y', 'z'] };
let catalogue = null;

/** The 26 predefined languages of JFLAP, asked once. */
async function languagesOf(tab) {
  if (!catalogue) catalogue = tab.api({ op: 'pumping_languages' }).catch((error) => ((catalogue = null), Promise.reject(error)));
  return catalogue;
}

const freshSeed = () => 1 + Math.floor(Math.random() * 1_000_000_000);

registerPanel({
  id: 'pumping-game',
  title: 'panel.pumping',
  appliesTo: ['regular_pumping_lemma', 'context_free_pumping_lemma'],
  order: 10,
  mount(root, tab) {
    const kind = KINDS[tab.type];
    const body = h('div', { class: 'panel-body pumping-panel' });
    root.append(body);
    let response = null; // the last engine answer (view, result, ...)
    let languages = [];
    let lengths = [];
    let chosenLanguage = null;
    let chosenMode = null;

    /** Plays a move; only an accepted one becomes the document (so undo steps back through the game). */
    async function play(move) {
      await guarded(tab, async () => {
        const r = await tab.api({ op: 'pumping_move', game: tab.doc, move });
        response = r;
        if (r.reply && r.reply.accepted) {
          await tab.commit(r.document);
        } else draw();
      });
    }

    /** The view of the document as it is (a rejected, harmless move returns it without changing anything). */
    async function refresh() {
      await guarded(tab, async () => {
        response = await tab.api({ op: 'pumping_move', game: tab.doc, move: { move: 'show_case', row: 9999 } });
        response.reply = null;
        draw();
      });
    }

    async function newGame() {
      await guarded(tab, async () => {
        const language = chosenLanguage ?? (currentLanguage(response.view.title, languages) || languages[0]).index;
        const r = await tab.api({ op: 'pumping_new', kind, language, mode: chosenMode || response.view.mode, seed: freshSeed() });
        response = r;
        lengths = [];
        await tab.commit(r.document);
      });
    }

    function numberBox(box, move, min = 0) {
      const input = h('input', { type: 'number', min, class: 'narrow', id: `pbox-${tab.id}-${box.number}`, 'aria-label': t('pumping.box', { n: box.number }) });
      const send = () => {
        const value = Number.parseInt(input.value, 10);
        if (Number.isFinite(value)) play(move(value));
      };
      input.addEventListener('keydown', (e) => e.key === 'Enter' && (e.preventDefault(), send()));
      return [input, h('button', { type: 'button', onclick: send }, t('pumping.enter'))];
    }

    function decompositionBox(view, box) {
      const w = view.boxes[1].content;
      const names = PART_NAMES[view.parts];
      lengths = clampLengths(w, lengths.length === view.parts ? lengths : Array(view.parts).fill(0), view.parts);
      const preview = h('p', { class: 'mono decomposition', 'aria-live': 'polite' });
      const rows = h('div', { class: 'decomposition-inputs' });
      const refreshPreview = () => {
        const parts = decompositionParts(w, lengths);
        clear(preview).append(...parts.map((part, i) => h('span', { class: `part part-${names[i] || 'z'}`, title: names[i] || 'z' }, h('small', {}, names[i] || 'z'), part === '' ? 'λ' : part)));
      };
      const sliders = [];
      const numbers = [];
      const sync = () => {
        lengths.forEach((length, i) => {
          const room = [...w].length - lengths.slice(0, i).reduce((a, b) => a + b, 0);
          sliders[i].max = String(room);
          sliders[i].value = String(length);
          numbers[i].max = String([...w].length);
          numbers[i].value = String(indicesFromLengths(lengths)[i]);
        });
        refreshPreview();
      };
      names.slice(0, view.parts).forEach((name, i) => {
        const slider = h('input', { type: 'range', min: 0, max: [...w].length, value: 0, 'aria-label': t('pumping.length_of', { part: name }), oninput: (e) => {
          lengths[i] = Number(e.target.value);
          lengths = clampLengths(w, lengths, view.parts);
          sync();
        } });
        const number = h('input', { type: 'number', min: 0, class: 'narrow', 'aria-label': t('pumping.end_of', { part: name }), onchange: (e) => {
          const cuts = indicesFromLengths(lengths);
          cuts[i] = Number(e.target.value) || 0;
          lengths = clampLengths(w, lengthsFromIndices(cuts), view.parts);
          sync();
        } });
        sliders.push(slider);
        numbers.push(number);
        rows.append(h('div', { class: 'form-row' }, h('label', {}, `${name}`), slider, number));
      });
      sync();
      return [preview, rows, h('button', { type: 'button', class: 'primary', onclick: () => play({ move: 'choose_decomposition', lengths: [...lengths] }) }, t('pumping.decompose'))];
    }

    function boxView(view, box) {
      const parts = [h('h4', {}, `${box.number}. `, richSpan(box.caption))];
      if (!box.visible) return null;
      if (box.editable) {
        if (box.input === 'number') parts.push(h('div', { class: 'button-row' }, numberBox(box, box.number === 1 ? (m) => ({ move: 'choose_m', m }) : (i) => ({ move: 'choose_i', i }))));
        else if (box.input === 'text') {
          const input = h('input', { type: 'text', class: 'mono wide', id: `pbox-${tab.id}-2`, spellcheck: 'false', autocomplete: 'off', 'aria-label': t('pumping.box', { n: 2 }) });
          const send = () => play({ move: 'choose_w', w: input.value.trim() });
          input.addEventListener('keydown', (e) => e.key === 'Enter' && (e.preventDefault(), send()));
          parts.push(h('div', { class: 'button-row' }, input, h('button', { type: 'button', onclick: send }, t('pumping.enter'))), h('small', {}, t('pumping.w_hint')));
        } else if (box.input === 'decomposition') parts.push(...decompositionBox(view, box));
      } else if (box.number === 5) {
        if (view.result) parts.push(h('p', { class: 'result-line', role: 'status' }, richSpan(view.result.message)));
        else if (box.content) parts.push(h('p', {}, richSpan(box.content)));
      } else if (box.content !== '') parts.push(h('p', { class: 'mono big-form' }, box.number === 3 ? richSpan(box.content) : box.content));
      if (box.message) parts.push(h('p', { class: 'form-error', role: 'alert' }, richSpan(box.message)));
      return h('section', { class: `pumping-box${box.editable ? ' active' : ''}`, 'aria-label': t('pumping.box', { n: box.number }) }, parts);
    }

    function casesView(view) {
      const cases = view.cases;
      let selected = cases.selected;
      const radios = cases.rows.map((row, i) => h('label', { class: 'case-row' }, h('input', { type: 'radio', name: `case-${tab.id}`, checked: i === selected, onchange: () => (selected = i) }), ' ', row));
      const needRow = (move) => () => (selected === null || selected === undefined ? tab.notify(t('pumping.case_select'), 'error') : play({ move, row: selected }));
      return h(
        'section',
        { class: 'pumping-cases', 'aria-label': t('pumping.cases') },
        h('h3', {}, t('pumping.cases')),
        h('p', { class: 'muted' }, t('pumping.cases_total', { n: cases.total })),
        cases.rows.length ? h('div', { role: 'radiogroup', 'aria-label': t('pumping.cases') }, radios) : h('p', { class: 'muted' }, t('pumping.cases_none')),
        cases.message ? h('p', { role: 'status' }, cases.message) : null,
        h(
          'div',
          { class: 'button-row' },
          h('button', { type: 'button', disabled: !cases.addReplaceEnabled, onclick: () => play({ move: 'add_case' }) }, t('pumping.case_add')),
          h('button', { type: 'button', disabled: !cases.addReplaceEnabled, onclick: needRow('replace_case') }, t('pumping.case_replace')),
          h('button', { type: 'button', onclick: needRow('show_case') }, t('pumping.case_show')),
          h('button', { type: 'button', onclick: needRow('delete_case') }, t('pumping.case_delete')),
          h('button', { type: 'button', disabled: !cases.listEnabled, onclick: () => play({ move: 'list_cases' }) }, t('pumping.case_list')),
          h('button', { type: 'button', onclick: () => play({ move: 'clear_cases' }) }, t('pumping.case_clear')),
          h('button', { type: 'button', onclick: () => play({ move: 'cases_done' }) }, t('pumping.case_done')),
        ),
      );
    }

    function draw() {
      clear(body);
      if (!response) return;
      const view = pumpingView(response, kind);
      const here = currentLanguage(view.title, languages);
      const languageSelect = h('select', { id: `plang-${tab.id}`, onchange: (e) => (chosenLanguage = Number(e.target.value)) }, languages.map((l) => h('option', { value: l.index, selected: here ? l.index === here.index : false }, l.title)));
      const modeSelect = h('select', { id: `pmode-${tab.id}`, onchange: (e) => (chosenMode = e.target.value) }, ['computer_first', 'human_first'].map((mode) => h('option', { value: mode, selected: mode === view.mode }, t(`pumping.mode.${mode}`))));
      chosenLanguage = here ? here.index : null;
      chosenMode = view.mode;
      body.append(
        h('h3', {}, richSpan(view.languageHtml)),
        h('div', { class: 'form-row' }, h('label', { for: languageSelect.id }, t('pumping.language')), languageSelect),
        h('div', { class: 'form-row' }, h('label', { for: modeSelect.id }, t('pumping.mode')), modeSelect),
        h('div', { class: 'button-row' }, h('button', { type: 'button', class: 'primary', onclick: newGame }, t('pumping.new_game')), h('button', { type: 'button', onclick: () => play({ move: 'clear_all' }) }, t('pumping.clear_all'))),
        h('p', { class: 'objective' }, h('strong', {}, `${t('pumping.objective')}: `), view.objective),
      );
      if (view.reply && !view.reply.accepted) body.append(h('p', { class: 'form-error', role: 'alert' }, t('pumping.reply_rejected', { message: view.reply.message })));
      for (const box of view.boxes) {
        const node = boxView(view, box);
        if (node) body.append(node);
      }
      if (view.result && view.result.pumped !== '') body.append(h('p', { class: 'mono' }, `${t('pumping.pumped')}: ${view.result.pumped}`));
      if (view.result) {
        body.append(h('p', { class: `outcome ${view.result.userWins ? 'outcome-accepted' : 'outcome-rejected'}`, role: 'status' }, view.result.userWins ? t('pumping.win_user') : t('pumping.win_computer')));
        if (view.result.takeaway) body.append(h('p', {}, richSpan(view.result.takeaway)));
      }
      if (view.conclusion) body.append(h('p', { class: 'conclusion', role: 'status' }, t(`pumping.conclusion.${view.conclusion}`)));
      if (view.attempts.length) body.append(h('h3', {}, t('pumping.attempts')), h('ol', { class: 'mono attempts' }, view.attempts.map((a) => h('li', {}, richSpan(a)))));
      if (view.cases) body.append(casesView(view));
      if (here) body.append(h('details', { class: 'pumping-explanation' }, h('summary', {}, t('pumping.explanation')), h('p', {}, richSpan(here.explanation))));
    }

    (async () => {
      try {
        languages = pumpingLanguages(await languagesOf(tab), kind);
      } catch (error) {
        tab.notify(error.message, 'error');
      }
      await refresh();
    })();
    const off = tab.on('doc', (e) => {
      if (e.cause === 'undo' || e.cause === 'redo' || e.cause === 'replace' || e.cause === 'host') refresh();
      else if (e.cause === 'edit' && response) draw();
    });
    return { destroy: off, focus: () => body.querySelector('input, select')?.focus() };
  },
});

async function newPumping(tab, kind) {
  await guarded(tab, async () => {
    const r = await tab.api({ op: 'pumping_new', kind, language: 0, mode: 'computer_first', seed: freshSeed() });
    tab.app.newTab(r.document);
  });
}

registerAction({ id: 'file.new.pumping_regular', menu: 'file', label: 'file.new_pumping_regular', order: 5.3, run: (tab) => newPumping(tab, 'regular') });
registerAction({ id: 'file.new.pumping_cf', menu: 'file', label: 'file.new_pumping_cf', order: 5.4, run: (tab) => newPumping(tab, 'context_free') });
