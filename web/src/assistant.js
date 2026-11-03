// "Assistant" panel: what the AI server put in `structuredContent` next to the document (an SVG, an
// analysis, a simulation, a design verification report). Also exports `valueView`, a readable rendering
// of any JSON value that other panels can reuse.

import { h } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { configTable } from './sim.js';
import { hasTable, markdownNodes } from './markdown.js';
import { contentExtras } from './tool-result.js';

const isPrimitive = (v) => v === null || typeof v !== 'object';

/** Readable DOM for any JSON value: objects as definition lists, arrays of objects as tables. */
export function valueView(value, depth = 0) {
  if (isPrimitive(value)) {
    if (typeof value === 'boolean') return h('span', { class: value ? 'badge ok' : 'badge bad' }, value ? t('word.yes') : t('word.no'));
    return h('span', {}, value === null || value === undefined ? '—' : value === '' ? 'λ' : String(value));
  }
  if (Array.isArray(value)) {
    if (!value.length) return h('span', { class: 'muted' }, '—');
    if (value.every(isPrimitive)) return h('span', {}, value.map((v) => (v === '' ? 'λ' : String(v))).join(', '));
    if (value.every((v) => v && typeof v === 'object' && !Array.isArray(v)) && depth < 3) {
      const keys = [...new Set(value.flatMap((v) => Object.keys(v)))];
      return h(
        'table',
        { class: 'configs' },
        h('thead', {}, h('tr', {}, keys.map((k) => h('th', { scope: 'col' }, k.replace(/_/g, ' '))))),
        h('tbody', {}, value.map((row) => h('tr', {}, keys.map((k) => h('td', {}, k in row ? valueView(row[k], depth + 1) : ''))))),
      );
    }
    return h('ol', {}, value.map((v) => h('li', {}, valueView(v, depth + 1))));
  }
  return h('dl', { class: 'facts' }, Object.entries(value).map(([k, v]) => [h('dt', {}, k.replace(/_/g, ' ')), h('dd', {}, valueView(v, depth + 1))]));
}

registerPanel({
  id: 'assistant',
  title: 'panel.assistant',
  appliesTo: ['*'],
  order: 5,
  visible: (tab) => Boolean(tab && tab.extras && Object.keys(contentExtras(tab.extras)).length),
  mount(root, tab) {
    const body = h('div', { class: 'panel-body' });
    root.append(body);

    function draw() {
      body.replaceChildren();
      const extras = contentExtras(tab.extras);
      if (extras.message) body.append(h('p', {}, extras.message));
      if (typeof extras.svg === 'string' && extras.svg.includes('<svg')) {
        // an <img> never runs scripts, whatever the SVG contains
        body.append(h('img', { class: 'assistant-svg', alt: t('assistant.svg_alt'), src: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(extras.svg)}` }));
      }
      if (extras.simulation) {
        const sim = extras.simulation;
        body.append(h('h3', {}, t('assistant.simulation')), h('p', { class: `outcome outcome-${sim.outcome}` }, t(`sim.${sim.outcome}`) === `sim.${sim.outcome}` ? String(sim.outcome) : t(`sim.${sim.outcome}`)));
        if (sim.output != null) body.append(h('p', {}, `${t('sim.col.output')}: ${sim.output === '' ? 'λ' : sim.output}`));
        if (Array.isArray(sim.trace) && sim.trace.length) body.append(configTable(tab.type, sim.trace));
      }
      if (extras.analysis) body.append(h('h3', {}, t('assistant.analysis')), valueView(extras.analysis));
      if (extras.verification) body.append(h('h3', {}, t('assistant.verification')), valueView(extras.verification));
      for (const [key, value] of Object.entries(extras)) {
        if (['message', 'svg', 'simulation', 'analysis', 'verification', 'document', 'steps'].includes(key)) continue;
        const markdown = typeof value === 'string' && (key === 'explanation' || hasTable(value));
        body.append(h('h3', {}, key.replace(/_/g, ' ')), markdown ? h('div', { class: 'markdown' }, markdownNodes(value)) : valueView(value));
      }
    }

    draw();
    return { destroy: tab.on('extras', draw) };
  },
});
