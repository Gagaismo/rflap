import test from 'node:test';
import assert from 'node:assert/strict';
import { Replay, replayKeyAction } from '../src/replay.js';
import { richTokens } from '../src/richtext.js';

test('replay: next / previous stop at the ends and say whether they moved', () => {
  const r = new Replay(3);
  assert.deepEqual([r.index, r.position, r.atStart, r.atEnd], [0, 1, true, false]);
  assert.equal(r.prev(), false);
  assert.equal(r.next(), true);
  assert.equal(r.next(), true);
  assert.equal(r.next(), false);
  assert.deepEqual([r.index, r.position, r.atStart, r.atEnd], [2, 3, false, true]);
  assert.equal(r.prev(), true);
  assert.equal(r.index, 1);
});

test('replay: first, last and "do all" jump; goTo clamps', () => {
  const r = new Replay(5);
  assert.equal(r.doAll(), true);
  assert.equal(r.index, 4);
  assert.equal(r.doAll(), false);
  assert.equal(r.first(), true);
  assert.equal(r.index, 0);
  assert.equal(r.goTo(99), true);
  assert.equal(r.index, 4);
  r.goTo(-7);
  assert.equal(r.index, 0);
  assert.equal(r.goTo(Number.NaN), false);
});

test('replay: an empty or one-step replay never moves, and reset keeps the index in range', () => {
  const empty = new Replay(0);
  assert.deepEqual([empty.next(), empty.prev(), empty.doAll(), empty.index, empty.atStart, empty.atEnd, empty.position], [false, false, false, 0, true, true, 0]);
  const one = new Replay(1);
  assert.deepEqual([one.next(), one.index, one.atEnd], [false, 0, true]);
  const r = new Replay(10);
  r.goTo(8);
  r.reset(4);
  assert.deepEqual([r.count, r.index], [4, 0]);
  r.goTo(3);
  r.reset(4, { keep: true });
  assert.equal(r.index, 3);
  r.reset(2, { keep: true });
  assert.equal(r.index, 1);
});

test('replay: keyboard actions', () => {
  const key = (k, extra = {}) => replayKeyAction({ key: k, ...extra });
  assert.equal(key('ArrowRight'), 'next');
  assert.equal(key('n'), 'next');
  assert.equal(key('ArrowLeft'), 'prev');
  assert.equal(key('p'), 'prev');
  assert.equal(key('Home'), 'first');
  assert.equal(key('End'), 'last');
  assert.equal(key('a'), 'last');
  assert.equal(key(' '), 'play');
  assert.equal(key('x'), null);
  assert.equal(key('ArrowRight', { ctrlKey: true }), null, 'shortcuts of the app are left alone');
  assert.equal(key('n', { altKey: true }), null);
});

test('richtext: the pumping lemma messages keep <i>, <sup>, <sub> and entities, and drop anything else', () => {
  assert.deepEqual(richTokens('<i>xy</i><sup>2</sup><i>z</i> = a'), [
    { text: 'xy', italic: true },
    { text: '2', sup: true },
    { text: 'z', italic: true },
    { text: ' = a' },
  ]);
  assert.deepEqual(richTokens('n<sub>a</sub> &#8800; n<sub>b</sub> &lt; &amp;&#x3bb;'), [
    { text: 'n' },
    { text: 'a', sub: true },
    { text: ' ≠ n' },
    { text: 'b', sub: true },
    { text: ' < &λ' },
  ]);
  assert.deepEqual(richTokens('a<br>b<script>alert(1)</script>'), [{ text: 'a' }, { br: true }, { text: 'balert(1)' }]);
  assert.deepEqual(richTokens(''), []);
  assert.deepEqual(richTokens(null), []);
});
