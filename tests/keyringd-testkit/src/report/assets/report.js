
const search = document.querySelector('#search');
const status = document.querySelector('#status');
const showEvidence = document.querySelector('#show-evidence');
const evidenceDetails = [...document.querySelectorAll('details.evidence-details')];
function updateEvidenceControl() {
  const openCount = evidenceDetails.filter(details => details.open).length;
  const state = openCount === 0 ? 'false' : openCount === evidenceDetails.length ? 'true' : 'mixed';
  showEvidence.setAttribute('aria-pressed', state);
  showEvidence.title = state === 'true' ? 'Collapse all evidence details' : 'Expand all evidence details';
}
showEvidence.disabled = evidenceDetails.length === 0;
updateEvidenceControl();
showEvidence.addEventListener('click', () => {
  const expand = showEvidence.getAttribute('aria-pressed') !== 'true';
  for (const details of evidenceDetails) details.open = expand;
  updateEvidenceControl();
});
for (const details of evidenceDetails) {
  details.addEventListener('toggle', updateEvidenceControl);
}
function filterCases() {
  const query = search.value.toLowerCase();
  for (const item of document.querySelectorAll('.case')) {
    const matchesText = item.dataset.search.includes(query);
    const matchesStatus = status.value === 'all' || item.dataset.status === status.value;
    item.hidden = !(matchesText && matchesStatus);
  }
  for (const group of document.querySelectorAll('.group')) {
    group.hidden = !group.querySelector('.case:not([hidden])');
  }
  for (const item of document.querySelectorAll('.toc-items > li')) {
    const link = item.querySelector('.toc-link');
    const target = link && document.getElementById(link.dataset.target);
    item.hidden = !target || target.hidden;
  }
  for (const file of document.querySelectorAll('.toc-file')) {
    const visibleTargets = file.dataset.targets
      .split(' ')
      .filter(Boolean)
      .filter(targetId => {
        const target = document.getElementById(targetId);
        return target && !target.hidden;
      });
    file.hidden = visibleTargets.length === 0;
    const fileLink = file.querySelector(':scope > .toc-file-link');
    if (fileLink && visibleTargets.length > 0) {
      fileLink.dataset.target = visibleTargets[0];
    }
  }
  const directories = [...document.querySelectorAll('.toc-tree details')].reverse();
  for (const directory of directories) {
    directory.hidden = !directory.querySelector(
      ':scope > .toc-file:not([hidden]), :scope > details:not([hidden])'
    );
  }
}
search.addEventListener('input', filterCases);
status.addEventListener('change', filterCases);
for (const item of document.querySelectorAll('.toc-link')) {
  item.addEventListener('click', () => {
    const target = document.getElementById(item.dataset.target);
    if (!target) return;
    search.value = '';
    status.value = 'all';
    filterCases();
    target.scrollIntoView({ behavior: 'smooth', block: 'start' });
  });
}

const language = document.querySelector('#language');
const translationStatus = document.querySelector('#translation-status');
const supportedTranslationLanguages = [
  'ar', 'bg', 'bn', 'cs', 'da', 'de', 'el', 'en', 'es', 'fi', 'fr', 'he', 'hi', 'hr',
  'hu', 'id', 'it', 'ja', 'kn', 'ko', 'lt', 'mr', 'nl', 'no', 'pl', 'pt', 'ro', 'ru',
  'sk', 'sl', 'sv', 'ta', 'te', 'th', 'tr', 'uk', 'vi', 'zh', 'zh-Hant'
];
const languageNames = new Intl.DisplayNames([navigator.language], { type: 'language' });
for (const languageTag of supportedTranslationLanguages) {
  const option = document.createElement('option');
  option.value = languageTag;
  option.textContent = languageNames.of(languageTag) || languageTag;
  language.appendChild(option);
}

const requestedLanguage = new URL(window.location.href).searchParams.get('lang');
if (requestedLanguage === 'original' || supportedTranslationLanguages.includes(requestedLanguage)) {
  language.value = requestedLanguage;
}

function updateLanguageQuery(targetLanguage) {
  const url = new URL(window.location.href);
  if (targetLanguage === 'original') {
    url.searchParams.delete('lang');
  } else {
    url.searchParams.set('lang', targetLanguage);
  }
  window.history.replaceState(null, '', url.href);
}

const originalTexts = new WeakMap();
for (const element of document.querySelectorAll('[data-translatable]')) {
  originalTexts.set(element, element.textContent);
}
const translationCache = new Map();
const translatorCache = new Map();
const pendingTranslations = new Map();
let translationVersion = 0;
let translationWorker = null;
let translationFailureVersion = -1;

function sourceLanguageFor(text) {
  return /[\u3040-\u30ff\u3400-\u9fff]/u.test(text) ? 'ja' : 'en';
}

function selectedLanguageName() {
  return language.selectedOptions[0]?.textContent || language.value;
}

function showTranslationError(error) {
  const message = error instanceof Error ? error.message : String(error);
  const retry = document.createElement('button');
  retry.type = 'button';
  retry.className = 'translation-retry';
  retry.textContent = message.includes('NotAllowed') ? 'Enable' : 'Retry';
  retry.title = `Enable ${selectedLanguageName()} translation`;
  retry.addEventListener('click', () => {
    prepareTranslators(language.value);
    translateReport();
  });
  translationStatus.replaceChildren(
    document.createTextNode(message.includes('NotAllowed') ? 'Translation needs confirmation. ' : `${message || 'Translation failed'} `),
    retry
  );
}

async function translatorFor(sourceLanguage, targetLanguage) {
  const key = `${sourceLanguage}\u0000${targetLanguage}`;
  if (translatorCache.has(key)) return translatorCache.get(key);
  if (!('Translator' in globalThis)) {
    throw new Error('Chrome Translator API is not available');
  }
  const version = translationVersion;
  const creation = Translator.create({
      sourceLanguage,
      targetLanguage,
      monitor(monitor) {
        monitor.addEventListener('downloadprogress', event => {
          if (language.value !== targetLanguage || version !== translationVersion) return;
          const percentage = Math.round(event.loaded * 100);
          translationStatus.textContent = `Downloading language model… ${percentage}%`;
        });
      }
    }).catch(error => {
      const detail = error instanceof Error ? `${error.name}: ${error.message}` : String(error);
      throw new Error(`Cannot create translator ${sourceLanguage} → ${targetLanguage} (${detail})`);
    });
  translatorCache.set(key, creation);
  creation.catch(() => {
    if (translatorCache.get(key) === creation) translatorCache.delete(key);
  });
  return creation;
}

function releaseTranslators() {
  for (const translator of translatorCache.values()) {
    translator.then(instance => instance.destroy()).catch(() => {});
  }
  translatorCache.clear();
}

function prepareTranslators(targetLanguage) {
  translationVersion += 1;
  releaseTranslators();
  if (targetLanguage === 'original' || !('Translator' in globalThis)) return;
  for (const sourceLanguage of ['ja', 'en']) {
    if (sourceLanguage !== targetLanguage) {
      // Start creation while the language-selection user gesture is still active.
      translatorFor(sourceLanguage, targetLanguage).catch(() => {});
    }
  }
}

function translateText(text, targetLanguage) {
  const sourceLanguage = sourceLanguageFor(text);
  if (sourceLanguage === targetLanguage) return Promise.resolve(text);
  const key = `${sourceLanguage}\u0000${targetLanguage}\u0000${text}`;
  if (translationCache.has(key)) return Promise.resolve(translationCache.get(key));
  const pendingKey = `${translationVersion}\u0000${key}`;
  if (pendingTranslations.has(pendingKey)) return pendingTranslations.get(pendingKey);
  const translation = translatorFor(sourceLanguage, targetLanguage)
    .then(translator => translator.translate(text))
    .then(value => {
      translationCache.set(key, value);
      return value;
    });
  pendingTranslations.set(pendingKey, translation);
  const finished = () => pendingTranslations.delete(pendingKey);
  translation.then(finished, finished);
  return translation;
}

function restoreCard(card) {
  for (const element of card.querySelectorAll('[data-translatable]')) {
    element.textContent = originalTexts.get(element);
  }
  const title = card.querySelector('.case-name');
  const copyButton = card.querySelector('.copy-case-title');
  if (title && copyButton) copyButton.dataset.copyText = originalTexts.get(title);
  card.dataset.translationLanguage = 'original';
}

async function translateCard(card, targetLanguage, version) {
  if (card.dataset.translationLanguage === targetLanguage) return;
  const elements = [...card.querySelectorAll('[data-translatable]')];
  const translations = await Promise.all(
    elements.map(element => translateText(originalTexts.get(element), targetLanguage))
  );
  if (language.value !== targetLanguage || version !== translationVersion) return;
  for (const [index, element] of elements.entries()) {
    element.textContent = translations[index];
  }
  const title = card.querySelector('.case-name');
  const copyButton = card.querySelector('.copy-case-title');
  if (title && copyButton) copyButton.dataset.copyText = title.textContent;
  card.dataset.translationLanguage = targetLanguage;
}

async function translateReport() {
  const targetLanguage = language.value;
  const cards = [...document.querySelectorAll('.case')];
  if (targetLanguage === 'original') {
    for (const card of cards) restoreCard(card);
    translationStatus.textContent = '';
    return;
  }
  if (!('Translator' in globalThis)) {
    translationStatus.textContent = 'Translator API is unavailable in this browser';
    return;
  }
  if (translationWorker || translationFailureVersion === translationVersion) return;
  const version = translationVersion;
  translationWorker = (async () => {
    try {
      while (version === translationVersion) {
        const pending = cards.filter(card => card.dataset.translationLanguage !== targetLanguage);
        const completed = cards.length - pending.length;
        if (!pending.length) {
          translationStatus.textContent = `Translated report to ${selectedLanguageName()} (${completed}/${cards.length})`;
          return;
        }
        // Prefer the current viewport, then continue from the top of the document.
        const margin = window.innerHeight * 0.75;
        const nearby = pending.find(card => {
          if (card.hidden) return false;
          const bounds = card.getBoundingClientRect();
          return bounds.bottom >= -margin && bounds.top <= window.innerHeight + margin;
        });
        translationStatus.textContent = `Translating report to ${selectedLanguageName()} (${completed}/${cards.length})…`;
        await translateCard(nearby || pending[0], targetLanguage, version);
        // Yield between cards so scrolling and language selection stay responsive.
        await new Promise(resolve => window.setTimeout(resolve, 0));
      }
    } catch (error) {
      if (version !== translationVersion) return;
      translationFailureVersion = version;
      console.error('Could not translate report', error);
      showTranslationError(error);
    }
  })();
  await translationWorker;
  translationWorker = null;
  if (version !== translationVersion) translateReport();
}

const caseObserver = new IntersectionObserver(() => {
  translateReport();
}, { rootMargin: '75% 0px', threshold: 0.01 });
for (const card of document.querySelectorAll('.case')) caseObserver.observe(card);
language.addEventListener('change', () => {
  updateLanguageQuery(language.value);
  prepareTranslators(language.value);
  translateReport();
});
if (language.value !== 'original') {
  prepareTranslators(language.value);
  translateReport();
}

for (const source of document.querySelectorAll('.case-code')) {
  source.addEventListener('toggle', () => {
    const card = source.closest('.case');
    for (const row of card?.querySelectorAll('.step-row') || []) {
      row.setAttribute('aria-expanded', String(source.open));
    }
    if (source.open && window.hljs) {
      const code = source.querySelector('code.language-rust:not(.hljs)');
      if (code) hljs.highlightElement(code);
    }
  });
}
function openTestSource(row) {
  const sourceId = row.getAttribute('aria-controls');
  const source = sourceId && document.getElementById(sourceId);
  if (source) source.open = true;
}
for (const row of document.querySelectorAll('.step-row')) {
  row.addEventListener('click', event => {
    if (event.target.closest('a, button')) return;
    openTestSource(row);
  });
  row.addEventListener('keydown', event => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    openTestSource(row);
  });
}
async function copyText(text) {
  if (navigator.clipboard && window.isSecureContext) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const input = document.createElement('textarea');
  input.value = text;
  input.setAttribute('readonly', '');
  input.style.position = 'fixed';
  input.style.opacity = '0';
  document.body.appendChild(input);
  input.select();
  const copied = document.execCommand('copy');
  input.remove();
  if (!copied) throw new Error('copy command was rejected');
}
for (const link of document.querySelectorAll('.case-code-source')) {
  link.addEventListener('click', event => event.stopPropagation());
}
for (const button of document.querySelectorAll('.copy-button')) {
  button.addEventListener('click', async event => {
    event.preventDefault();
    event.stopPropagation();
    try {
      await copyText(button.dataset.copyText);
      button.classList.add('copied');
      button.title = 'Copied';
      window.setTimeout(() => {
        button.classList.remove('copied');
        button.title = button.dataset.copyLabel;
      }, 1200);
    } catch (error) {
      console.error('Could not copy text', error);
      button.title = 'Copy failed';
    }
  });
}
