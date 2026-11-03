// Small DOM pieces the T12b panels share (tables, images of engine SVG, labelled rows).

import { h } from './dom.js';
import { t } from './i18n.js';

/** `<table class="configs">` with a caption for screen readers. `rows` are arrays of cells (nodes, strings or `{text, class}`). */
export function dataTable(caption, headers, rows, { rowClass = () => '', className = 'configs' } = {}) {
  const cell = (c, tag = 'td') => (c && typeof c === 'object' && !(c instanceof Node) && 'text' in c ? h(tag, { class: c.class || '', title: c.title }, c.text) : h(tag, {}, c));
  return h(
    'table',
    { class: className },
    caption ? h('caption', { class: 'sr-only' }, caption) : null,
    headers && headers.length ? h('thead', {}, h('tr', {}, headers.map((header) => h('th', { scope: 'col' }, header)))) : null,
    h('tbody', {}, rows.map((row, index) => h('tr', { class: rowClass(index, row) }, row.map((c) => cell(c))))),
  );
}

/** An engine SVG as an image: an <img> with a data URI never runs scripts, whatever the SVG contains. */
export function svgImage(svg, alt, className = 'engine-svg') {
  return h('img', { class: className, alt, src: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}` });
}

/** `<label>` + control in a row. */
export function labelled(label, control, hint) {
  const id = control.id || `w-${Math.random().toString(36).slice(2, 9)}`;
  control.id = id;
  return h('div', { class: 'form-row' }, h('label', { for: id }, label), h('div', { class: 'form-input' }, control), hint ? h('small', {}, hint) : null);
}

/** An `<input>` or `<textarea>` with a λ button next to it. */
export function withLambda(control) {
  const insert = () => {
    const start = control.selectionStart ?? control.value.length;
    const end = control.selectionEnd ?? control.value.length;
    control.value = control.value.slice(0, start) + 'λ' + control.value.slice(end);
    control.focus();
    control.setSelectionRange(start + 1, start + 1);
    control.dispatchEvent(new Event('input', { bubbles: true }));
  };
  return h('span', { class: 'with-lambda' }, control, h('button', { type: 'button', class: 'lambda', title: t('dialog.insert_lambda'), 'aria-label': t('dialog.insert_lambda'), onclick: insert }, 'λ'));
}

/** The accepted / rejected / error badge of a parse outcome. */
export function outcomeBadge(outcome) {
  const key = `parse.outcome.${outcome}`;
  return h('span', { class: `outcome outcome-${outcome === 'accepted' ? 'accepted' : outcome === 'rejected' ? 'rejected' : 'undecided'}` }, t(key) === key ? String(outcome) : t(key));
}

/** A line that says the same in words what a colour says: `ok` or not. */
export function statusLine(ok, okText, badText) {
  return h('p', { class: ok ? 'status-ok' : 'status-bad', role: 'status' }, h('span', { class: ok ? 'badge ok' : 'badge bad' }, ok ? '✓' : '✗'), ' ', ok ? okText : badText);
}
