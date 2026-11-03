import test from 'node:test';
import assert from 'node:assert/strict';
import { fitView, MIN_ZOOM, MAX_ZOOM } from '../src/fit.js';

const view = (w, h) => ({ width: w, height: h });

test('a box that fits the viewport is shown at 100% and centered', () => {
  const r = fitView({ minX: 0, minY: 0, maxX: 400, maxY: 200 }, view(500, 300), { margin: 24 });
  assert.deepEqual(r, { scale: 1, offsetX: -50, offsetY: -50 });
});

test('a tiny automaton is never zoomed in above 100%', () => {
  const r = fitView({ minX: 100, minY: 100, maxX: 110, maxY: 110 }, view(500, 300), { margin: 20 });
  assert.equal(r.scale, 1);
  assert.equal(r.offsetX, 105 - 250);
  assert.equal(r.offsetY, 105 - 150);
});

test('a big box is scaled down so that width and height fit with the margin', () => {
  const r = fitView({ minX: 0, minY: 0, maxX: 1000, maxY: 400 }, view(540, 240), { margin: 20 });
  assert.deepEqual(r, { scale: 0.5, offsetX: -40, offsetY: -40 });
});

test('the scale is limited by the tighter axis', () => {
  // width allows 1.0, height only 0.5
  const r = fitView({ minX: 0, minY: 0, maxX: 400, maxY: 400 }, view(1000, 240), { margin: 20 });
  assert.equal(r.scale, 0.5);
  assert.equal(r.offsetY, -20 / 0.5);
  assert.equal(r.offsetX, 200 - 1000 / 0.5 / 2);
});

test('the origin of the box is respected (negative and positive coordinates)', () => {
  const r = fitView({ minX: -200, minY: 50, maxX: 800, maxY: 450 }, view(540, 240), { margin: 20 });
  assert.equal(r.scale, 0.5);
  assert.equal(r.offsetX, -200 - 40);
  assert.equal(r.offsetY, 50 - 40);
});

test('the minimum zoom bounds the result and the box is anchored at the margin on the overflowing axis', () => {
  const r = fitView({ minX: 0, minY: 0, maxX: 10000, maxY: 10 }, view(500, 300), { margin: 20 });
  assert.equal(r.scale, MIN_ZOOM);
  assert.equal(r.offsetX, -20 / MIN_ZOOM);
  assert.equal(r.offsetY, -(300 / MIN_ZOOM - 10) / 2);
});

test('maxZoom can be raised or lowered by the caller', () => {
  const tiny = { minX: 0, minY: 0, maxX: 10, maxY: 10 };
  assert.equal(fitView(tiny, view(500, 300), { margin: 20, maxZoom: 2 }).scale, 2);
  assert.equal(fitView(tiny, view(500, 300), { margin: 20, maxZoom: 0.5 }).scale, 0.5);
  assert.equal(fitView(tiny, view(500, 300), { margin: 20 }).scale, 1);
  assert.ok(MAX_ZOOM >= 1);
});

test('an empty automaton keeps 100% at the origin', () => {
  assert.deepEqual(fitView(null, view(500, 300)), { scale: 1, offsetX: 0, offsetY: 0 });
  assert.deepEqual(fitView(undefined, view(0, 0)), { scale: 1, offsetX: 0, offsetY: 0 });
});

test('a single point (zero width and height) does not divide by zero', () => {
  const r = fitView({ minX: 40, minY: 40, maxX: 40, maxY: 40 }, view(500, 300), { margin: 20 });
  assert.equal(r.scale, 1);
  assert.equal(r.offsetX, 40 - 250);
  assert.equal(r.offsetY, 40 - 150);
});

test('a viewport without size gives the minimum zoom instead of NaN', () => {
  const r = fitView({ minX: 0, minY: 0, maxX: 100, maxY: 100 }, view(0, 0), { margin: 20 });
  assert.equal(r.scale, MIN_ZOOM);
  assert.ok(Number.isFinite(r.offsetX) && Number.isFinite(r.offsetY));
});

test('after the fit every corner of the box is inside the viewport, margin included', () => {
  for (const [w, h, box] of [
    [320, 200, { minX: -30, minY: -10, maxX: 1500, maxY: 900 }],
    [800, 600, { minX: 0, minY: 0, maxX: 300, maxY: 120 }],
    [200, 800, { minX: 10, minY: 10, maxX: 900, maxY: 60 }],
  ]) {
    const r = fitView(box, view(w, h), { margin: 16 });
    const left = (box.minX - r.offsetX) * r.scale;
    const top = (box.minY - r.offsetY) * r.scale;
    const right = (box.maxX - r.offsetX) * r.scale;
    const bottom = (box.maxY - r.offsetY) * r.scale;
    assert.ok(left >= 16 - 1e-9 && top >= 16 - 1e-9, `${w}x${h} top-left`);
    assert.ok(right <= w - 16 + 1e-9 && bottom <= h - 16 + 1e-9, `${w}x${h} bottom-right`);
  }
});
