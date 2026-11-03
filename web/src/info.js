// "Document" panel: the document of any type as readable JSON, to read or copy. The grammar, regex, L-system
// and pumping lemma environments have their own panels; this one is the last tab of every type.

import { h } from './dom.js';
import { t } from './i18n.js';
import { registerPanel } from './registry.js';
import { compactOf, isAutomaton } from './doc.js';

registerPanel({
  id: 'document',
  title: 'panel.document',
  appliesTo: ['*'],
  order: 90,
  mount(root, tab) {
    const area = h('textarea', { class: 'json', readonly: true, rows: 14, spellcheck: 'false', 'aria-label': t('panel.document') });
    root.append(h('div', { class: 'panel-body' }, h('p', { class: 'muted' }, t('document.hint', { type: tab.type })), area));
    const draw = () => {
      let shown = tab.doc;
      try {
        if (isAutomaton(tab.type)) shown = compactOf(tab.doc, { points: true });
      } catch {
        /* an inconsistent document is shown in its full form */
      }
      area.value = JSON.stringify(shown, null, 2);
    };
    draw();
    return { destroy: tab.on('doc', draw) };
  },
});
