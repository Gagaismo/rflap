// Modal dialogs, popup menus and toasts. Plain DOM (no <dialog>: it behaves differently across host iframes).

import { h } from './dom.js';
import { t } from './i18n.js';

let toastHost = null;

/** A transient message announced to screen readers (role=status). */
export function toast(message, kind = 'info') {
  if (!toastHost) {
    toastHost = h('div', { class: 'toasts', role: 'status', 'aria-live': 'polite' });
    document.body.append(toastHost);
  }
  const node = h('div', { class: `toast toast-${kind}` }, message);
  toastHost.append(node);
  setTimeout(() => node.remove(), kind === 'error' ? 7000 : 3500);
}

/**
 * Opens a modal. `body` is a node; `buttons` are `{label, primary, value}`; the promise resolves with the
 * value of the clicked button (or `null` on Escape / backdrop click). A button with `keepOpen` and `onClick`
 * runs the handler without closing. Focus is trapped inside while it is open and restored on close.
 */
export function openModal({ title, body, buttons = [], wide = false }) {
  const previous = document.activeElement;
  let finish;
  const promise = new Promise((resolve) => (finish = resolve));
  const close = (value = null) => {
    overlay.remove();
    document.removeEventListener('keydown', onKey, true);
    if (previous && previous.focus) previous.focus();
    finish(value);
  };
  const buttonRow = h(
    'div',
    { class: 'modal-buttons' },
    buttons.map((b) =>
      h('button', {
        type: 'button',
        class: b.primary ? 'primary' : '',
        text: b.label,
        onclick: () => (b.onClick ? b.onClick(close) : close(b.value)),
      }),
    ),
  );
  const box = h(
    'div',
    { class: `modal${wide ? ' modal-wide' : ''}`, role: 'dialog', 'aria-modal': 'true', 'aria-label': title },
    h('h2', { class: 'modal-title' }, title),
    body,
    buttonRow,
  );
  const overlay = h('div', { class: 'modal-backdrop', onmousedown: (e) => e.target === overlay && close(null) }, box);
  function onKey(e) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      close(null);
    } else if (e.key === 'Tab') {
      const items = [...box.querySelectorAll('button, input, select, textarea, [href]')].filter((n) => !n.disabled);
      if (!items.length) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }
  document.addEventListener('keydown', onKey, true);
  document.body.append(overlay);
  const focusable = box.querySelector('input, select, textarea') || box.querySelector('button.primary') || box.querySelector('button');
  if (focusable) focusable.focus();
  return { close, promise, box };
}

/**
 * A form dialog. `fields`: `{name, label, value, type: 'text'|'select'|'number'|'textarea', options: [[value, label]],
 * lambda: true (adds a λ button), placeholder, hint}`. `validate(values)` may be async and throw an Error whose
 * message is shown in the dialog (the dialog stays open). Resolves with the values or `null`.
 */
export function formDialog({ title, fields, submitLabel, intro, validate }) {
  const inputs = {};
  const error = h('p', { class: 'form-error', role: 'alert' });
  const rows = fields.map((f, i) => {
    const id = `f-${Date.now()}-${i}`;
    let input;
    if (f.type === 'select') {
      input = h('select', { id }, f.options.map(([v, label]) => h('option', { value: v, selected: String(v) === String(f.value) }, label)));
    } else if (f.type === 'textarea') {
      input = h('textarea', { id, rows: f.rows || 4, value: f.value ?? '', placeholder: f.placeholder });
    } else {
      input = h('input', { id, type: f.type || 'text', value: f.value ?? '', placeholder: f.placeholder, min: f.min, max: f.max, autocomplete: 'off', spellcheck: 'false' });
    }
    inputs[f.name] = input;
    const lambda = f.lambda
      ? h('button', { type: 'button', class: 'lambda', title: t('dialog.insert_lambda'), 'aria-label': t('dialog.insert_lambda'), onclick: () => insert(input, 'λ') }, 'λ')
      : null;
    return h('div', { class: 'form-row' }, h('label', { for: id }, f.label), h('div', { class: 'form-input' }, input, lambda), f.hint ? h('small', {}, f.hint) : null);
  });
  const form = h('form', { onsubmit: (e) => e.preventDefault() }, intro ? h('p', {}, intro) : null, rows, error);
  const read = () => Object.fromEntries(Object.entries(inputs).map(([k, el]) => [k, el.value]));
  const submit = async (close) => {
    error.textContent = '';
    const values = read();
    try {
      if (validate) {
        const replacement = await validate(values);
        close(replacement === undefined ? values : replacement);
        return;
      }
      close(values);
    } catch (e) {
      error.textContent = e && e.message ? e.message : String(e);
    }
  };
  const modal = openModal({
    title,
    body: form,
    buttons: [
      { label: t('dialog.cancel'), value: null },
      { label: submitLabel || t('dialog.ok'), primary: true, keepOpen: true, onClick: submit },
    ],
  });
  form.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target.tagName !== 'TEXTAREA' && e.target.tagName !== 'BUTTON') {
      e.preventDefault();
      submit(modal.close);
    }
  });
  return modal.promise;
}

function insert(input, text) {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  input.value = input.value.slice(0, start) + text + input.value.slice(end);
  input.focus();
  input.setSelectionRange(start + text.length, start + text.length);
}

export function messageDialog({ title, body, wide = false }) {
  return openModal({
    title,
    body: typeof body === 'string' ? h('p', {}, body) : body,
    buttons: [{ label: t('dialog.close'), primary: true, value: true }],
    wide,
  }).promise;
}

export async function confirmDialog({ title, message }) {
  const answer = await openModal({
    title,
    body: h('p', {}, message),
    buttons: [
      { label: t('dialog.cancel'), value: false },
      { label: t('dialog.ok'), primary: true, value: true },
    ],
  }).promise;
  return answer === true;
}

/** Shows text the browser would not let us copy or download; the user can select and copy it. */
export function textDialog({ title, text, hint }) {
  const area = h('textarea', { rows: 6, readonly: true, class: 'share-text', 'aria-label': title }, text);
  const modal = openModal({
    title,
    body: h('div', {}, hint ? h('p', {}, hint) : null, area),
    buttons: [{ label: t('dialog.close'), primary: true, value: true }],
    wide: true,
  });
  area.focus();
  area.select();
  return modal.promise;
}

/**
 * A popup menu at viewport coordinates. `items`: `{label, onSelect, disabled, checked, separator, shortcut}`.
 * Arrow keys, Enter and Escape work; any outside click closes it.
 */
export function popupMenu(x, y, items, { onClose, label } = {}) {
  const previous = document.activeElement;
  const menu = h('ul', { class: 'popup-menu', role: 'menu', 'aria-label': label || '' });
  const entries = [];
  for (const item of items) {
    if (item.separator) {
      menu.append(h('li', { role: 'separator', class: 'sep' }));
      continue;
    }
    const li = h(
      'li',
      {
        role: item.checked === undefined ? 'menuitem' : 'menuitemcheckbox',
        'aria-checked': item.checked === undefined ? null : String(Boolean(item.checked)),
        'aria-disabled': item.disabled ? 'true' : null,
        tabindex: '-1',
        class: item.disabled ? 'disabled' : '',
        onclick: () => {
          if (item.disabled) return;
          close();
          item.onSelect();
        },
      },
      h('span', { class: 'check', 'aria-hidden': 'true' }, item.checked ? '✓' : ''),
      h('span', { class: 'label' }, item.label),
      item.shortcut ? h('span', { class: 'shortcut' }, item.shortcut) : null,
    );
    entries.push(li);
    menu.append(li);
  }
  function close() {
    menu.remove();
    document.removeEventListener('mousedown', onOutside, true);
    document.removeEventListener('keydown', onKey, true);
    if (previous && previous.focus && document.body.contains(previous)) previous.focus();
    if (onClose) onClose();
  }
  function onOutside(e) {
    if (!menu.contains(e.target)) close();
  }
  function onKey(e) {
    const enabled = entries.filter((n) => n.getAttribute('aria-disabled') !== 'true');
    const at = enabled.indexOf(document.activeElement);
    if (e.key === 'Escape') {
      e.stopPropagation();
      close();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      enabled[(at + 1) % enabled.length]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      enabled[(at - 1 + enabled.length) % enabled.length]?.focus();
    } else if (e.key === 'Enter' || e.key === ' ') {
      if (at >= 0) {
        e.preventDefault();
        enabled[at].click();
      }
    } else if (e.key === 'Tab') {
      close();
    }
  }
  document.body.append(menu);
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(0, Math.min(x, window.innerWidth - rect.width - 4))}px`;
  menu.style.top = `${Math.max(0, Math.min(y, window.innerHeight - rect.height - 4))}px`;
  document.addEventListener('mousedown', onOutside, true);
  document.addEventListener('keydown', onKey, true);
  const first = entries.find((n) => n.getAttribute('aria-disabled') !== 'true');
  if (first) first.focus();
  return { close };
}

