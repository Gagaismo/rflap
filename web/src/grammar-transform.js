// Grammar environment, part 2: the transformations (remove λ, unit and useless productions, Chomsky normal
// form) replayed step by step in JFLAP's order. The steps come from the engine's `convert`.

import { h, clear } from './dom.js';
import { t, registerStrings } from './i18n.js';
import { registerPanel, registerAction } from './registry.js';
import { Replay, replayBar, replayKeys } from './replay.js';
import { grammarFrames, describeGrammarStep, productionText } from './grammar-model.js';
import { guarded } from './actions.js';

registerStrings({
  en: {
    'panel.grammar_transform': 'Transform',
    'transform.intro': 'Pick a transformation and step through it, like JFLAP\'s "Transform Grammar". Keys: n / → next, p / ← previous, End do all.',
    'transform.lambda': 'Remove λ-productions',
    'transform.unit': 'Remove unit productions',
    'transform.useless': 'Remove useless productions',
    'transform.cnf': 'Chomsky normal form',
    'transform.phase.lambda': 'Step 1: remove λ-productions',
    'transform.phase.unit': 'Step 2: remove unit productions',
    'transform.phase.useless': 'Step 3: remove useless productions',
    'transform.phase.cnf': 'Step 4: Chomsky normal form',
    'transform.phase_single': 'Phase: {name}',
    'transform.result': 'Result of this phase',
    'transform.renamed': 'Variables renamed: {renames}.',
    'transform.removed': 'Removed:',
    'transform.productions': 'Productions',
    'transform.mark.added': 'added',
    'transform.mark.focus': 'in use',
    'transform.open': 'Open the result in a new tab',
    'transform.apply': 'Replace this grammar with the result',
    'transform.none': 'Nothing to transform: the grammar has no productions.',
  },
  es: {
    'panel.grammar_transform': 'Transformar',
    'transform.intro': 'Elegí una transformación y recorrela paso a paso, como "Transform Grammar" de JFLAP. Teclas: n / → siguiente, p / ← anterior, Fin hacer todo.',
    'transform.lambda': 'Eliminar producciones λ',
    'transform.unit': 'Eliminar producciones unitarias',
    'transform.useless': 'Eliminar producciones inútiles',
    'transform.cnf': 'Forma normal de Chomsky',
    'transform.phase.lambda': 'Paso 1: eliminar producciones λ',
    'transform.phase.unit': 'Paso 2: eliminar producciones unitarias',
    'transform.phase.useless': 'Paso 3: eliminar producciones inútiles',
    'transform.phase.cnf': 'Paso 4: forma normal de Chomsky',
    'transform.phase_single': 'Fase: {name}',
    'transform.result': 'Resultado de esta fase',
    'transform.renamed': 'Variables renombradas: {renames}.',
    'transform.removed': 'Quitadas:',
    'transform.productions': 'Producciones',
    'transform.mark.added': 'agregada',
    'transform.mark.focus': 'en uso',
    'transform.open': 'Abrir el resultado en una pestaña nueva',
    'transform.apply': 'Reemplazar esta gramática con el resultado',
    'transform.none': 'No hay nada que transformar: la gramática no tiene producciones.',
  },
});

const OPERATIONS = [
  ['remove_lambda', 'transform.lambda', 'removeLambda'],
  ['remove_unit', 'transform.unit', 'removeUnit'],
  ['remove_useless', 'transform.useless', 'removeUseless'],
  ['cnf', 'transform.cnf', 'cnf'],
];

registerPanel({
  id: 'grammar-transform',
  title: 'panel.grammar_transform',
  appliesTo: ['grammar'],
  order: 20,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    const replay = new Replay(0);
    let run = null; // { to, result, frames }
    let bar = null;

    async function start(to) {
      if (!tab.doc.productions.length) {
        tab.notify(t('transform.none'), 'error');
        return;
      }
      await guarded(tab, async () => {
        const result = await tab.api({ op: 'convert', document: tab.doc, to });
        const frames = grammarFrames(result, tab.doc.productions);
        run = { to, result, frames };
        replay.reset(frames.length);
        draw();
        body.querySelector('.replay-region')?.focus();
      });
    }

    function frameView(frame) {
      const heading = run.to === 'cnf' ? t(`transform.phase.${frame.phase}`) : t(`transform.${frame.phase}`);
      const parts = [h('h3', {}, heading)];
      parts.push(h('p', { class: 'step-sentence', role: 'status' }, frame.step ? describeGrammarStep(frame.step) : t('transform.result')));
      if (frame.step === null && frame.renames && frame.renames.length) parts.push(h('p', { class: 'muted' }, t('transform.renamed', { renames: frame.renames.map(([a, b]) => `${a} → ${b}`).join(', ') })));
      if (frame.removed.length) parts.push(h('p', { class: 'removed' }, `${t('transform.removed')} `, frame.removed.map((p) => h('del', { class: 'mono' }, `${productionText(p)}  `))));
      parts.push(
        h(
          'ul',
          { class: 'production-list', 'aria-label': t('transform.productions') },
          frame.productions.map((p) => h('li', { class: `mono prod-${p.mark}` }, productionText(p), p.mark === 'keep' ? null : h('span', { class: 'prod-note' }, ` (${t(`transform.mark.${p.mark}`)})`))),
        ),
      );
      return parts;
    }

    function draw() {
      if (bar) bar.stop();
      clear(body);
      body.append(
        h('p', { class: 'muted' }, t('transform.intro')),
        h('div', { class: 'button-row' }, OPERATIONS.map(([to, label]) => h('button', { type: 'button', class: run && run.to === to ? 'primary' : '', onclick: () => start(to) }, t(label)))),
      );
      if (!run) return;
      bar = replayBar(replay, draw, { key: `transform-${tab.id}` });
      const region = h('div', { class: 'replay-region', 'aria-label': t('panel.grammar_transform') }, bar.node, frameView(run.frames[replay.index]));
      replayKeys(region, bar.act);
      body.append(
        region,
        h(
          'div',
          { class: 'button-row' },
          h('button', { type: 'button', onclick: () => tab.openTab(run.result.document, { title: `${tab.title} → ${t(OPERATIONS.find((o) => o[0] === run.to)[1])}` }) }, t('transform.open')),
          h('button', { type: 'button', onclick: () => tab.commit(run.result.document).then(() => { run = null; draw(); }) }, t('transform.apply')),
        ),
      );
    }

    draw();
    const off = tab.on('doc', (e) => {
      if (e.cause !== 'edit' || !run) return;
      run = null;
      draw();
    });
    const api = { destroy: () => (bar && bar.stop(), off()) };
    for (const [to, , method] of OPERATIONS) api[method] = () => start(to);
    return api;
  },
});

const MENU_ORDER = { remove_lambda: 15, remove_unit: 16, remove_useless: 17, cnf: 18 };
for (const [to, label, method] of OPERATIONS) {
  registerAction({ id: `convert.step.${to}`, menu: 'convert', label, appliesTo: ['grammar'], order: MENU_ORDER[to], run: (tab) => tab.app.showPanel(tab, 'grammar-transform', method) });
}
