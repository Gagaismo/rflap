// "Analysis" panel (JFLAP Test menu): kind of automaton, nondeterminism, λ transitions, unreachable and
// useless states, plus the text description of the machine (an accessible alternative to the drawing).

import { h, clear } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { debounce } from './dom.js';
import { machineDocument } from './doc.js';
import { multipistaOf } from './tm-model.js';

const yesNo = (b) => (b ? t('word.yes') : t('word.no'));
const list = (items) => (items.length ? items.join(', ') : '—');

registerPanel({
  id: 'analysis',
  title: 'panel.analysis',
  appliesTo: ['automata'],
  order: 40,
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);
    let seq = 0;

    const flag = (name, label) =>
      h('label', { class: 'inline' }, h('input', { type: 'checkbox', checked: tab.flags[name], onchange: (e) => tab.setFlag(name, e.target.checked) }), label);

    async function draw() {
      const mine = ++seq;
      let info;
      let text;
      try {
        const machine = machineDocument(tab.doc, tab.path);
        [info, text] = await Promise.all([tab.api({ op: 'analyze', document: machine }), tab.api({ op: 'render', document: machine, format: 'text' })]);
      } catch (error) {
        if (mine === seq) clear(body).append(h('p', { class: 'form-error', role: 'alert' }, error.message));
        return;
      }
      if (mine !== seq) return;
      const rows = [
        [t('analysis.kind'), t(`analysis.kind.${info.kind}`)],
        [t('analysis.alphabet'), list(info.alphabet || [])],
        [t('analysis.deterministic'), yesNo(info.deterministic)],
      ];
      if ('complete' in info) rows.push([t('analysis.complete'), yesNo(info.complete)]);
      rows.push(
        [t('analysis.nondeterministic'), list(info.nondeterministic_states || [])],
        [t('analysis.lambda'), list((info.lambda_transitions || []).map((l) => `${l.from} → ${l.to}`))],
        [t('analysis.unreachable'), list(info.unreachable_states || [])],
        [t('analysis.useless'), list(info.useless_states || [])],
      );
      const multi = multipistaOf(info);
      if (multi) {
        rows.push([t('analysis.multipista'), multi.multipista ? t('word.yes') : t('analysis.multipista_not', { list: multi.offending.map((i) => i + 1).join(', ') })]);
      }
      if ((info.catedra_warnings || []).length) rows.push([t('analysis.catedra_warnings'), info.catedra_warnings.join(' ')]);
      clear(body).append(
        h('div', { class: 'button-row' }, flag('nondeterminism', t('test.highlight_nondeterminism')), flag('lambda', t('test.highlight_lambda'))),
        h('dl', { class: 'facts' }, rows.map(([k, v]) => [h('dt', {}, k), h('dd', {}, v)])),
        h('h3', {}, t('analysis.description')),
        h('pre', { class: 'description', tabindex: '0' }, text.content),
      );
    }

    const redraw = debounce(draw, 80);
    draw();
    const offs = [tab.on('doc', (e) => (e.cause === 'flags' ? undefined : redraw())), tab.on('flags', () => draw())];
    return { destroy: () => offs.forEach((off) => off()), refresh: draw };
  },
});
