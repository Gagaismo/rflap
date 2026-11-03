import test from 'node:test';
import assert from 'node:assert/strict';
import { layoutTree, treeYield } from '../src/tree.js';
import { fixture } from './fixtures.js';

const parse = fixture('parse');
const tree = parse.ll1.results[0].tree;

test('tree layout: leaves go left to right, parents sit above the middle of their children', () => {
  const layout = layoutTree(tree);
  assert.equal(layout.nodes.length, 8);
  assert.equal(layout.edges.length, 7);
  const root = layout.nodes[0];
  assert.equal(root.symbol, 'S');
  assert.equal(root.y < layout.nodes[1].y, true);
  const leaves = layout.nodes.filter((n) => n.leaf);
  assert.deepEqual(leaves.map((n) => n.symbol), ['a', 'a', 'λ', 'b', 'b']);
  for (let i = 1; i < leaves.length; i++) assert.equal(leaves[i].x > leaves[i - 1].x, true, `leaf ${i}`);
  const children = layout.nodes.filter((n) => layout.edges.some((e) => e.from === 0 && e.to === n.id));
  assert.equal(root.x, (children[0].x + children[children.length - 1].x) / 2);
  assert.equal(layout.width >= Math.max(...layout.nodes.map((n) => n.x)), true);
  assert.equal(layout.height >= Math.max(...layout.nodes.map((n) => n.y)), true);
  assert.equal(layout.nodes.every((n) => n.x >= 0 && n.y >= 0), true);
});

test('tree layout: no node overlaps another on the same level', () => {
  const layout = layoutTree(parse.cyk.results[0].tree);
  const byLevel = new Map();
  for (const n of layout.nodes) byLevel.set(n.y, [...(byLevel.get(n.y) || []), n.x]);
  for (const xs of byLevel.values()) {
    const sorted = [...xs].sort((a, b) => a - b);
    for (let i = 1; i < sorted.length; i++) assert.equal(sorted[i] - sorted[i - 1] >= 20, true);
  }
});

test('tree: the yield is the derived string (λ leaves vanish); a single node is a tree too', () => {
  assert.equal(treeYield(tree), 'aabb');
  assert.equal(treeYield(parse.cyk.results[0].tree), 'ab');
  const single = layoutTree({ symbol: 'S', children: [] });
  assert.equal(single.nodes.length, 1);
  assert.equal(single.edges.length, 0);
  assert.equal(single.nodes[0].leaf, true);
  assert.equal(treeYield({ symbol: 'S', children: [] }), 'S');
});

test('tree: a partial tree of the user parse marks the variables still to expand', () => {
  const layout = layoutTree(parse.user.tree);
  assert.deepEqual(layout.nodes.filter((n) => n.open).map((n) => n.symbol), ['S']);
  assert.equal(layout.nodes[0].open, false, 'the root has children');
});
