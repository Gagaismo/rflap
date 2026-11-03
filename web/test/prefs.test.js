import test from 'node:test';
import assert from 'node:assert/strict';

// minimal browser stand-ins: the App only touches these when it is not mounted
const memory = new Map();
const storage = { getItem: (k) => (memory.has(k) ? memory.get(k) : null), setItem: (k, v) => memory.set(k, String(v)) };
globalThis.window = { localStorage: storage };
globalThis.document = { documentElement: { dataset: {}, style: { setProperty() {} }, lang: '' } };
Object.defineProperty(globalThis, 'navigator', { value: { language: 'en-US', languages: ['en-US', 'en'] }, configurable: true });
globalThis.matchMedia = () => ({ matches: true, addEventListener() {} }); // the browser prefers dark

const { App } = await import('../src/app.js');
const { i18n, setLanguage } = await import('../src/i18n.js');

const makeApp = (host = {}) => new App(null, { api: null, host });

test('the host theme "dark" (MCP Apps and ChatGPT shapes) never changes the classic look', () => {
  const app = makeApp();
  const contexts = [
    { theme: 'dark', locale: 'en-US', styles: { variables: { '--color-background-primary': '#111' } }, displayMode: 'inline' },
    { theme: 'dark', locale: 'en', widgetState: null },
    { theme: 'dark' },
  ];
  for (const context of contexts) {
    app.handleContext(context);
    assert.equal(app.dark, false);
    assert.equal(document.documentElement.dataset.theme, 'light');
  }
});

test('prefers-color-scheme: dark is ignored too', () => {
  const app = makeApp();
  app.applyTheme();
  assert.equal(app.dark, false);
  assert.equal(document.documentElement.dataset.theme, 'light');
});

test('a stored dark theme from an older version does not switch dark on', () => {
  memory.set('rflap.theme', 'dark');
  const app = makeApp();
  app.handleContext({});
  assert.equal(app.dark, false);
  assert.equal(document.documentElement.dataset.theme, 'light');
  memory.clear();
});

test('the language is Spanish by default, whatever the browser or the host say', () => {
  memory.clear();
  setLanguage('en');
  const app = makeApp();
  app.applyLanguageChoice();
  assert.equal(app.languageChoice, 'es');
  assert.equal(i18n.locale, 'es');
  app.handleContext({ locale: 'en' });
  app.handleContext({ locale: 'en-US', theme: 'dark' });
  assert.equal(i18n.locale, 'es');
  assert.equal(document.documentElement.lang, 'es');
});

test('the legacy "auto" language setting means Spanish', () => {
  memory.set('rflap.language', 'auto');
  const app = makeApp();
  assert.equal(app.languageChoice, 'es');
  memory.clear();
});

test('English is selectable, remembered in localStorage and restored by a new session', () => {
  memory.clear();
  const app = makeApp();
  app.setLanguage('en');
  assert.equal(i18n.locale, 'en');
  assert.equal(memory.get('rflap.language'), 'en');
  const next = makeApp();
  next.applyLanguageChoice();
  assert.equal(next.languageChoice, 'en');
  assert.equal(i18n.locale, 'en');
  next.setLanguage('es');
  assert.equal(memory.get('rflap.language'), 'es');
  assert.equal(i18n.locale, 'es');
  next.setLanguage('fr'); // not supported: falls back to Spanish
  assert.equal(next.languageChoice, 'es');
});

test('localStorage that throws does not break the choice', () => {
  const saved = window.localStorage;
  window.localStorage = {
    getItem() {
      throw new Error('blocked');
    },
    setItem() {
      throw new Error('blocked');
    },
  };
  try {
    const app = makeApp();
    assert.equal(app.languageChoice, 'es');
    app.setLanguage('en');
    assert.equal(i18n.locale, 'en');
  } finally {
    window.localStorage = saved;
    setLanguage('es');
  }
});

test('ChatGPT: the language is saved in the widgetState and restored from it', async () => {
  memory.clear();
  const saved = [];
  const host = { mode: 'chatgpt', updateModelContext: async (payload) => saved.push(payload) };
  const app = makeApp(host);
  app.setLanguage('en');
  assert.equal(saved.length, 1);
  assert.equal(saved[0].structured.language, 'en');

  memory.clear();
  const fresh = makeApp({ mode: 'chatgpt' });
  fresh.handleContext({ theme: 'dark', locale: 'es', widgetState: { language: 'en', document: {} } });
  assert.equal(fresh.languageChoice, 'en');
  assert.equal(i18n.locale, 'en');
  setLanguage('es');
});
