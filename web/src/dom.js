// Tiny DOM helpers (no framework).

const SVG_NS = 'http://www.w3.org/2000/svg';

/**
 * `h('button', {class: 'x', onclick: fn, 'aria-label': 'y', dataset: {k: 'v'}}, 'text', child)`.
 * Props starting with `on` are event listeners; `class`, `text`, `html` (trusted markup only) and `dataset`
 * are special; `false`/`null`/`undefined` props are skipped; `true` sets an empty attribute.
 */
export function h(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  apply(node, props);
  append(node, children);
  return node;
}

export function svgEl(tag, props = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  apply(node, props);
  append(node, children);
  return node;
}

function apply(node, props) {
  for (const [key, value] of Object.entries(props || {})) {
    if (value === false || value == null) continue;
    if (key === 'class') node.setAttribute('class', value);
    else if (key === 'text') node.textContent = value;
    else if (key === 'html') node.innerHTML = value;
    else if (key === 'dataset') Object.assign(node.dataset, value);
    else if (key.startsWith('on')) node.addEventListener(key.slice(2), value);
    else if (key === 'value' || key === 'checked' || key === 'selected' || key === 'disabled') node[key] = value;
    else node.setAttribute(key, value === true ? '' : String(value));
  }
}

function append(node, children) {
  for (const child of children.flat(Infinity)) {
    if (child == null || child === false) continue;
    node.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
}

/** `node.append(...)` that skips `null` / `false` (native `append(null)` writes the text "null") and flattens arrays. */
export function put(node, ...children) {
  append(node, children);
  return node;
}

export function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
  return node;
}

/** Saves text/bytes as a file download. Returns false when the environment cannot download. */
export function download(filename, content, mime = 'text/plain') {
  try {
    const blob = new Blob([content], { type: `${mime};charset=utf-8` });
    const url = URL.createObjectURL(blob);
    const link = h('a', { href: url, download: filename });
    document.body.append(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 2000);
    return true;
  } catch {
    return false;
  }
}

/** Debounce: the last call wins after `ms` of silence. */
export function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}
