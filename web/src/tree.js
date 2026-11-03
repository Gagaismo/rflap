// Parse trees: `{symbol, children: [...]}` as the engine returns them. The layout is pure (tested in node);
// `treeSvg` draws it as an accessible SVG.

import { svgEl } from './dom.js';
import { t, registerStrings } from './i18n.js';

registerStrings({ en: { 'tree.label': 'Parse tree' }, es: { 'tree.label': 'Árbol de derivación' } });

const TREE_LAMBDA = 'λ';

/**
 * Leaves go left to right with a fixed gap, every parent is centered over its first and last child, and
 * the depth sets the row. Returns `{nodes: [{id, symbol, x, y, leaf, open}], edges: [{from, to}], width, height}`.
 * `open` marks a variable without children in a partial tree (the user parse still has to expand it).
 */
export function layoutTree(tree, { charWidth = 9, minLeaf = 34, gap = 14, rowHeight = 56, pad = 24, note = (node) => node.note } = {}) {
  const nodes = [];
  const edges = [];
  let cursor = pad;
  let deepest = 0;
  const widthOf = (node) => Math.max(minLeaf, [...String(node.symbol)].length * charWidth + 12, [...String(note(node) || '')].length * (charWidth - 2) + 12);
  function place(node, depth, parent) {
    const id = nodes.length;
    const entry = { id, symbol: node.symbol, x: 0, y: pad + depth * rowHeight, leaf: !node.children || node.children.length === 0, open: false, note: note(node) || '', width: widthOf(node) };
    nodes.push(entry);
    if (parent !== null) edges.push({ from: parent, to: id });
    deepest = Math.max(deepest, depth);
    if (entry.leaf) {
      entry.open = isVariable(node.symbol);
      entry.x = cursor + entry.width / 2;
      cursor += entry.width + gap;
    } else {
      const placed = node.children.map((child) => place(child, depth + 1, id));
      entry.x = (nodes[placed[0]].x + nodes[placed[placed.length - 1]].x) / 2;
    }
    return id;
  }
  if (tree) place(tree, 0, null);
  return { nodes, edges, width: cursor - gap + pad, height: pad * 2 + deepest * rowHeight };
}

const isVariable = (symbol) => typeof symbol === 'string' && symbol !== TREE_LAMBDA && /^\p{Lu}/u.test(symbol);

/** The terminals of the leaves, left to right, without λ. A tree that is only a root yields the root symbol. */
export function treeYield(tree) {
  if (!tree) return '';
  if (!tree.children || tree.children.length === 0) return tree.symbol === TREE_LAMBDA ? '' : tree.symbol;
  return tree.children.map(treeYield).join('');
}

/** SVG of a layout. `label` is the accessible name (for example the derivation). */
export function treeSvg(layout, { label = '', highlight = null } = {}) {
  const svg = svgEl('svg', { class: 'parse-tree', role: 'img', 'aria-label': label || t('tree.label'), width: layout.width, height: layout.height, viewBox: `0 0 ${layout.width} ${layout.height}` });
  const at = (id) => layout.nodes[id];
  for (const edge of layout.edges) svg.append(svgEl('line', { x1: at(edge.from).x, y1: at(edge.from).y + 12, x2: at(edge.to).x, y2: at(edge.to).y - 12, class: 'tree-edge' }));
  for (const node of layout.nodes) {
    const classes = ['tree-node', node.leaf ? 'leaf' : 'inner', node.open ? 'open' : '', highlight && highlight.has(node.id) ? 'hot' : ''].filter(Boolean).join(' ');
    const group = svgEl('g', { class: classes, transform: `translate(${node.x} ${node.y})` });
    group.append(svgEl('rect', { x: -node.width / 2, y: -13, width: node.width, height: 26, rx: node.leaf ? 13 : 4 }), svgEl('text', { 'text-anchor': 'middle', y: 5 }, node.symbol));
    if (node.note) group.append(svgEl('text', { 'text-anchor': 'middle', y: 24, class: 'tree-note' }, node.note));
    svg.append(group);
  }
  return svg;
}
