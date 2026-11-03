// Simulation panels (JFLAP Input menu): step with closure / step by state, fast run, multiple run.
// The engine is stateless: "step n" asks for the frontier after n steps from the start, so Back is just n-1.

import { h, clear } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { tapeCount } from './doc.js';
import { acceptanceUsed, catedraBatchRequest, catedraRows, inputHasDelimiters } from './tm-model.js';

const MAX_ROWS = 200;

/** Settings shared by the simulation panels of a tab. */
export function simSettings(tab) {
  if (!tab.simSettings) {
    tab.simSettings = { mode: 'closure', acceptance: 'final_state', inputs: [''] };
  }
  const tapes = tapeCount(tab.doc);
  while (tab.simSettings.inputs.length < tapes) tab.simSettings.inputs.push('');
  tab.simSettings.inputs.length = tapes;
  return tab.simSettings;
}

/** The request fields that describe the input(s) and options of a run. */
export function runArguments(tab, inputs = simSettings(tab).inputs) {
  const s = simSettings(tab);
  const args = {};
  if (tapeCount(tab.doc) > 1) args.inputs = inputs;
  else args.input = inputs[0] || '';
  if (tab.type === 'fa' || tab.type === 'pda') args.mode = s.mode;
  if (tab.type === 'pda' || tab.type === 'turing') args.acceptance = s.acceptance;
  return args;
}

const blankLabel = (s) => (s === '' || s == null ? 'λ' : s);

/** A tape with its head: the cell under the head is marked. */
export function tapeView(tape) {
  const cells = [...tape.content];
  const head = tape.head;
  const node = h('span', { class: 'tape', role: 'text', 'aria-label': t('sim.tape_aria', { content: tape.content, head }) });
  if (!cells.length) cells.push('□');
  cells.forEach((c, i) => node.append(h('span', { class: i === head ? 'cell head' : 'cell' }, c)));
  if (head >= cells.length) node.append(h('span', { class: 'cell head' }, '□'));
  return node;
}

/** Table of configurations (frontier or trace) for the document type. */
export function configTable(type, configs, { onPick } = {}) {
  const columns = [];
  const sample = configs[0] || {};
  columns.push([t('sim.col.state'), (c) => c.state]);
  if (type === 'turing') columns.push([t('sim.col.tapes'), (c) => h('div', {}, (c.tapes || []).map(tapeView))]);
  else columns.push([t('sim.col.remaining'), (c) => blankLabel(c.remaining)]);
  if ('stack' in sample) columns.push([t('sim.col.stack'), (c) => blankLabel(c.stack)]);
  if ('output' in sample) columns.push([t('sim.col.output'), (c) => blankLabel(c.output)]);
  const table = h(
    'table',
    { class: 'configs' },
    h('thead', {}, h('tr', {}, columns.map(([title]) => h('th', { scope: 'col' }, title)))),
    h(
      'tbody',
      {},
      configs.slice(0, MAX_ROWS).map((c, i) =>
        h(
          'tr',
          onPick ? { tabindex: '0', onclick: () => onPick(c, i), onkeydown: (e) => (e.key === 'Enter' ? onPick(c, i) : null) } : {},
          columns.map(([, cell]) => h('td', {}, cell(c))),
        ),
      ),
    ),
  );
  if (configs.length > MAX_ROWS) return h('div', {}, table, h('p', { class: 'muted' }, t('sim.more', { n: configs.length - MAX_ROWS })));
  return table;
}

/** Root-level state ids of configurations (a Turing config inside a block reports the block's state first). */
export function configStates(configs) {
  return [...new Set(configs.map((c) => (c.path && c.path.length ? c.path[0] : c.state_id)).filter((x) => x != null))];
}

function outcomeBox(outcome, extra) {
  const label = { accepted: t('sim.accepted'), rejected: t('sim.rejected'), undecided: t('sim.undecided') }[outcome] || outcome;
  return h('p', { class: `outcome outcome-${outcome}`, role: 'status' }, label, extra ? ` ${extra}` : '');
}

registerPanel({
  id: 'simulate',
  title: 'panel.simulate',
  appliesTo: ['automata'],
  order: 10,
  mount(container, tab) {
    const settings = simSettings(tab);
    let steps = 0;
    let animation = null;
    const inputs = settings.inputs.map((value, i) =>
      h('input', {
        type: 'text',
        value,
        class: 'sim-input',
        'aria-label': tapeCount(tab.doc) > 1 ? t('sim.input_tape', { n: i + 1 }) : t('sim.input'),
        placeholder: tapeCount(tab.doc) > 1 ? t('sim.input_tape', { n: i + 1 }) : t('sim.input'),
        spellcheck: 'false',
        autocomplete: 'off',
        oninput: (e) => {
          settings.inputs[i] = e.target.value;
          resetRun();
        },
        onkeydown: (e) => {
          if (e.key === 'Enter') (e.shiftKey ? fastRun : stepForward)();
        },
      }),
    );
    const select = (key, values, label, onchange) =>
      h(
        'label',
        { class: 'inline' },
        label,
        h('select', { onchange: (e) => onchange(e.target.value) }, values.map((v) => h('option', { value: v, selected: settings[key] === v }, t(`sim.${key}.${v}`)))),
      );
    const options = h(
      'div',
      { class: 'sim-options' },
      tab.type === 'fa' || tab.type === 'pda' ? select('mode', ['closure', 'step_by_state'], t('sim.mode'), (v) => ((settings.mode = v), resetRun())) : null,
      tab.type === 'pda' ? select('acceptance', ['final_state', 'empty_stack'], t('sim.acceptance'), (v) => ((settings.acceptance = v), resetRun())) : null,
      tab.type === 'turing' ? select('acceptance', ['final_state', 'halting', 'both'], t('sim.acceptance'), (v) => ((settings.acceptance = v), resetRun())) : null,
    );
    const result = h('div', { class: 'sim-result', 'aria-live': 'polite' });
    const backButton = h('button', { type: 'button', onclick: () => stepBack(), disabled: true }, t('sim.back'));
    const buttons = h(
      'div',
      { class: 'button-row' },
      h('button', { type: 'button', class: 'primary', onclick: () => stepForward() }, t('sim.step')),
      backButton,
      h('button', { type: 'button', onclick: () => fastRun() }, t('sim.run')),
      h('button', { type: 'button', onclick: () => resetRun(true) }, t('sim.reset')),
    );
    container.append(h('div', { class: 'panel-body' }, h('div', { class: 'sim-inputs' }, inputs), options, buttons, result));

    function stopAnimation() {
      if (animation) clearInterval(animation);
      animation = null;
    }

    function resetRun(focus = false) {
      stopAnimation();
      steps = 0;
      backButton.disabled = true;
      tab.setOverlay('sim', null);
      clear(result);
      if (focus && inputs[0]) inputs[0].focus();
    }

    async function showStep() {
      stopAnimation();
      backButton.disabled = steps <= 0;
      try {
        const r = await tab.api({ op: 'step', document: tab.doc, steps, ...runArguments(tab) });
        clear(result);
        const heading = h('p', { class: 'muted' }, t('sim.after_steps', { n: r.steps_done }));
        result.append(heading);
        if (r.frontier.length) {
          result.append(h('h3', {}, t('sim.configs', { n: r.frontier.length })), configTable(tab.type, r.frontier, { onPick: (c) => tab.setOverlay('sim', { states: configStates([c]) }) }));
        }
        if (r.truncated) result.append(h('p', { class: 'warn' }, t('sim.truncated')));
        for (const a of r.accepted) result.append(outcomeBox('accepted'), configTable(tab.type, [a.config || a]));
        if (r.finished) {
          if (!r.accepted.length) result.append(outcomeBox('rejected'));
        } else if (!r.frontier.length) {
          result.append(outcomeBox('rejected'));
        }
        tab.setOverlay('sim', { states: configStates([...r.frontier, ...r.accepted.map((a) => a.config || a)]) });
        steps = r.steps_done;
      } catch (error) {
        clear(result).append(h('p', { class: 'form-error', role: 'alert' }, error.message));
      }
    }

    function stepForward() {
      steps += 1;
      return showStep();
    }

    function stepBack() {
      steps = Math.max(0, steps - 1);
      return showStep();
    }

    async function fastRun() {
      stopAnimation();
      try {
        const r = await tab.api({ op: 'simulate', document: tab.doc, ...runArguments(tab) });
        clear(result);
        const extra = r.output != null ? `${t('sim.col.output')}: ${blankLabel(r.output)}` : '';
        result.append(outcomeBox(r.outcome, extra));
        if (r.tape_outputs) result.append(h('div', {}, r.tape_outputs.map((o, i) => h('p', {}, `${t('sim.tape', { n: i + 1 })}: ${o}`))));
        if (r.trace && r.trace.length) {
          const play = h('button', { type: 'button', onclick: () => playTrace(r.trace) }, t('sim.play_trace'));
          result.append(h('h3', {}, t('sim.trace', { n: r.trace.length })), play, configTable(tab.type, r.trace, { onPick: (c) => tab.setOverlay('sim', { states: configStates([c]) }) }));
          tab.setOverlay('sim', { states: configStates([r.trace[r.trace.length - 1]]) });
        } else {
          tab.setOverlay('sim', null);
        }
      } catch (error) {
        clear(result).append(h('p', { class: 'form-error', role: 'alert' }, error.message));
      }
    }

    function playTrace(trace) {
      stopAnimation();
      let i = 0;
      animation = setInterval(() => {
        tab.setOverlay('sim', { states: configStates([trace[i]]) });
        i += 1;
        if (i >= trace.length) stopAnimation();
      }, 600);
    }

    const off = tab.on('doc', (e) => {
      if (e.cause !== 'path' && e.cause !== 'flags') {
        const tapes = tapeCount(tab.doc);
        if (tapes !== inputs.length) tab.notify(t('sim.tapes_changed'), 'info');
        resetRun();
      }
    });
    return {
      destroy() {
        stopAnimation();
        off();
      },
      focus() {
        inputs[0].focus();
      },
      step: stepForward,
      run: fastRun,
    };
  },
});

registerPanel({
  id: 'multiple',
  title: 'panel.multiple',
  appliesTo: ['automata'],
  order: 20,
  mount(container, tab) {
    const tapes = () => tapeCount(tab.doc);
    const rows = [Array.from({ length: tapes() }, () => '')];
    const body = h('tbody');
    const paste = h('textarea', { rows: 4, 'aria-label': t('multi.paste'), placeholder: t('multi.paste_hint') });

    const isAll = () => tab.type === 'turing' && tab.model === 'all';

    function rowViews(row, index, result) {
      const mark = h('tr', {},
        row.map((value, tape) => {
          const warning = h('small', { class: 'warn', role: 'status' }, tab.type === 'turing' && inputHasDelimiters(value) ? t('sim.delimiters_warning') : '');
          return h(
            'td',
            {},
            h('input', {
              type: 'text',
              value,
              spellcheck: 'false',
              'aria-label': t('multi.row_input', { n: index + 1 }),
              oninput: (e) => {
                row[tape] = e.target.value;
                warning.textContent = tab.type === 'turing' && inputHasDelimiters(e.target.value) ? t('sim.delimiters_warning') : '';
              },
            }),
            warning,
          );
        }),
        h('td', { class: result ? `outcome-${result.outcome}` : '' }, result ? t(`sim.${result.outcome}`) : ''),
        h('td', {}, result && result.output != null ? blankLabel(result.output) : result && result.tape_outputs ? result.tape_outputs.join(' | ') : ''),
        h('td', {}, h('button', { type: 'button', 'aria-label': t('multi.remove_row', { n: index + 1 }), onclick: () => ((rows.splice(index, 1), draw())) }, '✕')),
      );
      const notes = result && result.notes && result.notes.length ? result.notes : result && result.message ? [result.message] : [];
      if (!notes.length) return [mark];
      const span = row.length + 3;
      return [mark, h('tr', { class: 'notes' }, h('td', { colspan: String(span) }, notes.map((n) => h('p', { class: 'muted' }, `ℹ️ ${n}`))))];
    }

    function draw(results = []) {
      clear(body);
      rows.forEach((row, i) => body.append(...rowViews(row, i, results[i])));
    }

    function showCriterion() {
      criterion.textContent = tab.type === 'turing' ? t('multi.criterion', { criterion: t(`sim.acceptance.${acceptanceUsed(tab.model, simSettings(tab).acceptance)}`) }) : '';
    }

    async function runAll() {
      const settings = simSettings(tab);
      const list = rows.filter((r) => r.some((x) => x !== '') || rows.length === 1);
      try {
        const inputs = list.map((r) => (tapes() > 1 ? r : r[0]));
        let results;
        if (isAll()) {
          // an ALL runs on ⟨w⟩ and accepts by final state: the cátedra engine, not the JFLAP simulation
          const r = await tab.api(catedraBatchRequest(tab.doc, inputs, 'all', catedraMaxSteps()));
          results = catedraRows(r.results);
        } else {
          const args = { op: 'multiple_run', document: tab.doc, inputs };
          if (tab.type === 'fa' || tab.type === 'pda') args.mode = settings.mode;
          if (tab.type === 'pda' || tab.type === 'turing') args.acceptance = acceptanceUsed(tab.model, settings.acceptance);
          results = (await tab.api(args)).results;
        }
        rows.length = 0;
        list.forEach((l) => rows.push(l));
        draw(results);
        showCriterion();
        const accepted = results.filter((x) => x.accepted).length;
        summary.textContent = t('multi.summary', { accepted, total: results.length });
      } catch (error) {
        summary.textContent = error.message;
      }
    }

    function loadPasted() {
      const lines = paste.value.split('\n').filter((l, i, all) => l !== '' || i < all.length - 1);
      if (!lines.length) return;
      rows.length = 0;
      for (const line of lines) {
        const parts = tapes() > 1 ? line.split('|') : [line];
        rows.push(Array.from({ length: tapes() }, (_, i) => parts[i] || ''));
      }
      paste.value = '';
      draw();
    }

    const summary = h('p', { role: 'status', 'aria-live': 'polite', class: 'muted' });
    const criterion = h('p', { class: 'muted', role: 'status' });
    const catedraMaxSteps = () => (tab.catedra && tab.catedra.maxSteps) || 1000;
    const settingsBar = tab.type === 'turing' ? h('div', { class: 'sim-options' }) : null;
    function drawBar() {
      if (!settingsBar) return;
      const settings = simSettings(tab);
      const modelSelect = h('select', { 'aria-label': t('catedra.model'), onchange: (e) => tab.setModel(e.target.value) }, ['mt', 'all'].map((m) => h('option', { value: m, selected: tab.model === m }, t(`catedra.model_${m}`))));
      const acceptance = h(
        'select',
        { 'aria-label': t('sim.acceptance'), disabled: tab.model === 'all', onchange: (e) => ((settings.acceptance = e.target.value), showCriterion()) },
        ['final_state', 'halting', 'both'].map((v) => h('option', { value: v, selected: acceptanceUsed(tab.model, settings.acceptance) === v }, t(`sim.acceptance.${v}`))),
      );
      clear(settingsBar).append(h('label', { class: 'inline' }, t('catedra.model'), modelSelect), h('label', { class: 'inline' }, t('sim.acceptance'), acceptance));
    }
    const head = h('tr', {}, Array.from({ length: tapes() }, (_, i) => h('th', { scope: 'col' }, tapes() > 1 ? t('sim.input_tape', { n: i + 1 }) : t('sim.input'))), h('th', { scope: 'col' }, t('multi.result')), h('th', { scope: 'col' }, t('sim.col.output')), h('th', {}));
    container.append(
      h(
        'div',
        { class: 'panel-body' },
        settingsBar,
        h('table', { class: 'configs multi' }, h('thead', {}, head), body),
        h(
          'div',
          { class: 'button-row' },
          h('button', { type: 'button', onclick: () => (rows.push(Array.from({ length: tapes() }, () => '')), draw()) }, t('multi.add_row')),
          h('button', { type: 'button', class: 'primary', onclick: runAll }, t('multi.run_all')),
        ),
        summary,
        criterion,
        h('details', {}, h('summary', {}, t('multi.paste')), paste, h('button', { type: 'button', onclick: loadPasted }, t('multi.load'))),
      ),
    );
    drawBar();
    draw();
    showCriterion();
    const offModel = tab.on('model', () => {
      drawBar();
      draw();
      summary.textContent = '';
      showCriterion();
    });
    const off = tab.on('doc', (e) => {
      if (e.cause === 'path' || e.cause === 'flags') return;
      const n = tapes();
      for (const row of rows) {
        while (row.length < n) row.push('');
        row.length = n;
      }
      draw();
    });
    return { destroy: () => (off(), offModel()), run: runAll };
  },
});
