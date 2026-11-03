import test from 'node:test';
import assert from 'node:assert/strict';
import { createI18n, pickLocale, dictionaries, registerStrings } from '../src/i18n.js';

test('pickLocale takes the first supported language of the candidates', () => {
  assert.equal(pickLocale(['es-AR', 'en-US']), 'es');
  assert.equal(pickLocale(['fr-FR', 'es-ES', 'en']), 'es');
  assert.equal(pickLocale(['EN-gb']), 'en');
  assert.equal(pickLocale(['fr']), 'en');
  assert.equal(pickLocale([]), 'en');
  assert.equal(pickLocale([undefined, null, 'es']), 'es');
});

test('both dictionaries have exactly the same keys', () => {
  const own = (d) => Object.keys(d).filter((k) => !k.startsWith('test.')).sort();
  const en = own(dictionaries.en);
  const es = own(dictionaries.es);
  assert.deepEqual(es, en);
  assert.ok(en.length > 100);
});

test('text differs by language', () => {
  assert.notEqual(createI18n('en').t('menu.file'), createI18n('es').t('menu.file'));
  assert.equal(createI18n('es').t('menu.file'), 'Archivo');
});

test('falls back to English, then to the key', () => {
  registerStrings({ en: { 'test.only_en': 'only English', 'test.hello': 'Hello {name}, {n} items' } });
  const es = createI18n('es');
  assert.equal(es.t('test.only_en'), 'only English');
  assert.equal(es.t('test.missing_everywhere'), 'test.missing_everywhere');
  assert.equal(es.t('test.hello', { name: 'Ana', n: 3 }), 'Hello Ana, 3 items');
});

test('an unknown locale behaves as English and the locale can change', () => {
  const i18n = createI18n('fr');
  assert.equal(i18n.locale, 'en');
  i18n.setLocale('es-AR');
  assert.equal(i18n.locale, 'es');
  assert.equal(i18n.t('menu.file'), 'Archivo');
});

test('the app-wide instance starts in Spanish', async () => {
  const { i18n } = await import('../src/i18n.js');
  assert.equal(i18n.locale, 'es');
  assert.equal(i18n.t('menu.file'), 'Archivo');
});

test('every English string has a real Spanish translation (identical text only for names, symbols and cognates)', async () => {
  // load every panel so that their registerStrings() are in the dictionaries
  for (const file of ['actions', 'sim', 'structure', 'analysis', 'steps', 'assistant', 'info', 'grammar-editor', 'grammar-transform', 'grammar-tables', 'grammar-parse', 'regex-panel', 'design-panel', 'lsystem-panel', 'pumping-panel']) {
    await import(`../src/${file}.js`);
  }
  const SAME_ON_PURPOSE = new Set([
    'convert.combined', 'layout.gem', 'word.final', 'word.no', 'state.final', 'multi.paste_hint', 'tables.col.variable',
    'parse.parser.ll1', 'parse.parser.slr1', 'parse.col.n', 'parse.error', 'pumping.enter', 'pumping.m', 'pumping.w', 'pumping.i',
  ]);
  const same = Object.keys(dictionaries.en).filter((k) => !k.startsWith('test.') && dictionaries.es[k] === dictionaries.en[k]);
  const untranslated = same.filter((k) => !SAME_ON_PURPOSE.has(k) && !/^(help\.sc\..*\.keys|design\.template\.|.*\.keys)/.test(k));
  assert.deepEqual(untranslated, []);
});
