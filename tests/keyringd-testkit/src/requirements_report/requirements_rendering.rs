use super::{
    Ledger, ReportError, ReportPaths, Requirement, RequirementScope, RequirementStatus,
    TestOutcome, TestResult,
};

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use maud::{DOCTYPE, PreEscaped, html};

pub fn generate(paths: &ReportPaths) -> Result<(), ReportError> {
    let yaml = fs::read_to_string(&paths.requirements).map_err(|source| ReportError::Read {
        path: paths.requirements.clone(),
        source,
    })?;
    let ledger: Ledger = serde_saphyr::from_str(&yaml).map_err(|source| ReportError::Yaml {
        path: paths.requirements.clone(),
        source: Box::new(source),
    })?;
    validate(&ledger)?;
    let mut results = load_test_results(&paths.test_results)?;
    let requirements_path = if paths.requirements.is_absolute() {
        paths.requirements.clone()
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(&paths.requirements))
            .unwrap_or_else(|_| paths.requirements.clone())
    };
    let workspace_root = requirements_path.parent();
    for result in results.values_mut() {
        result.source.file = display_source_path(&result.source.file, workspace_root);
    }
    let platform = paths
        .test_results
        .file_name()
        .and_then(|name| name.to_str());
    let html = render(&ledger, &results, platform);
    if let Some(parent) = paths.output.parent() {
        fs::create_dir_all(parent).map_err(|source| ReportError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&paths.output, html).map_err(|source| ReportError::Write {
        path: paths.output.clone(),
        source,
    })?;
    Ok(())
}

fn validate(ledger: &Ledger) -> Result<(), ReportError> {
    let mut requirements = HashSet::new();
    for (index, requirement) in ledger.requirements.iter().enumerate() {
        let number = index + 1;
        if requirement.source.title.trim().is_empty() {
            return Err(ReportError::Invalid(format!(
                "requirement {number} has an empty source title"
            )));
        }
        let url = requirement.source.url.trim();
        if !(url.starts_with("https://") || url.starts_with("http://")) || !url.contains('#') {
            return Err(ReportError::Invalid(format!(
                "requirement {number} source URL must be an HTTP(S) URL with a fragment"
            )));
        }
        if requirement.text.trim().is_empty() {
            return Err(ReportError::Invalid(format!(
                "requirement {number} has empty text"
            )));
        }
        if requirement.scope.requires_reason()
            && requirement
                .reason
                .as_deref()
                .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(ReportError::Invalid(format!(
                "requirement {number} with scope {} requires a reason",
                requirement.scope.label()
            )));
        }
        let key = (url, requirement.text.trim());
        if !requirements.insert(key) {
            return Err(ReportError::Invalid(format!(
                "requirement {number} duplicates an earlier source and text"
            )));
        }
        let mut test_ids = HashSet::new();
        for test_id in &requirement.tests {
            if test_id.trim().is_empty() || !test_ids.insert(test_id) {
                return Err(ReportError::Invalid(format!(
                    "requirement {number} contains an empty or duplicate test ID"
                )));
            }
        }
    }
    Ok(())
}

fn load_test_results(directory: &Path) -> Result<BTreeMap<String, TestResult>, ReportError> {
    let mut results = BTreeMap::new();
    if !directory.exists() {
        return Ok(results);
    }
    let entries = fs::read_dir(directory).map_err(|source| ReportError::Read {
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let path = entry
            .map_err(|source| ReportError::Read {
                path: directory.to_path_buf(),
                source,
            })?
            .path();
        if !crate::infrastructure::ResultStore::is_result_file(&path) {
            continue;
        }
        let json = fs::read(&path).map_err(|source| ReportError::Read {
            path: path.clone(),
            source,
        })?;
        let result: TestResult =
            serde_json::from_slice(&json).map_err(|source| ReportError::Json {
                path: path.clone(),
                source,
            })?;
        if results.insert(result.id.clone(), result).is_some() {
            return Err(ReportError::Invalid(format!(
                "duplicate current test result ID in {}",
                directory.display()
            )));
        }
    }
    Ok(results)
}

// Application crates only run on their target OS. Shared crates must still
// report missing results; a different platform must never turn a gap into PASS.
fn test_applies_to_platform(id: &str, platform: Option<&str>) -> bool {
    let Some(platform @ ("linux" | "windows" | "macos")) = platform else {
        return true;
    };
    match id.split("::").next().unwrap_or(id) {
        "keyringd-linux" | "keyringd-wsl" => platform == "linux",
        "keyringd-windows" => platform == "windows",
        "keyringd-macos" => platform == "macos",
        _ => true,
    }
}

fn display_source_path(file: &str, workspace_root: Option<&Path>) -> String {
    let path = Path::new(file);
    let display_path = workspace_root
        .and_then(|root| path.strip_prefix(root).ok())
        .or_else(|| (!path.is_absolute()).then_some(path))
        .or_else(|| path.file_name().map(Path::new))
        .unwrap_or(path);
    display_path.to_string_lossy().replace('\\', "/")
}

fn requirement_status(
    requirement: &Requirement,
    results: &BTreeMap<String, TestResult>,
    platform: Option<&str>,
) -> RequirementStatus {
    match requirement.scope {
        RequirementScope::OtherComponent => return RequirementStatus::OtherComponent,
        RequirementScope::OutOfScope => return RequirementStatus::OutOfScope,
        RequirementScope::Deferred => return RequirementStatus::Deferred,
        RequirementScope::In | RequirementScope::Boundary => {}
    }
    if requirement.tests.is_empty() {
        return RequirementStatus::Uncovered;
    }
    let matched = requirement
        .tests
        .iter()
        .filter(|id| test_applies_to_platform(id, platform))
        .map(|id| results.get(id))
        .collect::<Vec<_>>();
    if matched.is_empty() {
        return RequirementStatus::OtherPlatform;
    }
    if matched.iter().any(|result| result.is_none()) {
        return RequirementStatus::MissingResult;
    }
    let matched = matched.into_iter().flatten().collect::<Vec<_>>();
    if matched
        .iter()
        .any(|result| result.outcome == TestOutcome::Failed)
    {
        return RequirementStatus::Failed;
    }
    if matched
        .iter()
        .any(|result| result.outcome == TestOutcome::Skipped)
    {
        return RequirementStatus::Skipped;
    }
    RequirementStatus::Covered
}

fn render(
    ledger: &Ledger,
    results: &BTreeMap<String, TestResult>,
    platform: Option<&str>,
) -> String {
    let statuses = ledger
        .requirements
        .iter()
        .map(|requirement| requirement_status(requirement, results, platform))
        .collect::<Vec<_>>();
    let count = |status| statuses.iter().filter(|value| **value == status).count();
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width,initial-scale=1";
                title { "keyringd requirements report" }
                style { (PreEscaped(STYLE)) }
            }
            body {
                div class="layout" {
                    aside class="toc" aria-label="Requirement sources" {
                        h2 { "Sources" }
                        nav {
                            @for (index, requirement) in ledger.requirements.iter().enumerate() {
                                a href=(format!("#requirement-{index}")) title=(&requirement.source.title) {
                                    (&requirement.source.title)
                                }
                            }
                        }
                    }
                    main {
                        header class="summary" {
                            div {
                                h1 { "Requirements report" }
                                p {
                                (ledger.requirements.len()) " requirements · "
                                (count(RequirementStatus::Covered)) " covered · "
                                (count(RequirementStatus::Uncovered)) " uncovered · "
                                (count(RequirementStatus::Failed)) " failed · "
                                (count(RequirementStatus::MissingResult)) " missing results"
                                }
                            }
                            div class="translation" {
                                span id="translation-status" class="translation-status" aria-live="polite" {}
                                label class="translation-control" for="language" {
                                    span { "Language" }
                                    select id="language" {
                                        option value="original" { "Original" }
                                    }
                                }
                            }
                        }
                        div class="toolbar" {
                            input id="search" type="search" placeholder="Filter requirements, sources, and tests";
                            select id="status" {
                                option value="all" { "All statuses" }
                                @for status in [
                                    RequirementStatus::Covered,
                                    RequirementStatus::Uncovered,
                                    RequirementStatus::Failed,
                                    RequirementStatus::Skipped,
                                    RequirementStatus::MissingResult,
                                    RequirementStatus::OtherPlatform,
                                    RequirementStatus::OtherComponent,
                                    RequirementStatus::OutOfScope,
                                    RequirementStatus::Deferred,
                                ] {
                                    option value=(status.css_class()) { (status.label()) }
                                }
                            }
                        }
                        @for (index, requirement) in ledger.requirements.iter().enumerate() {
                            @let status = statuses[index];
                            @let test_search = requirement.tests.iter().map(|id| {
                                results.get(id).map_or(id.as_str(), |result| result.title.as_str())
                            }).collect::<Vec<_>>().join(" ");
                            article
                                id=(format!("requirement-{index}"))
                                class="requirement"
                                data-status=(status.css_class())
                                data-search=(format!("{} {} {}", requirement.source.title, requirement.text, test_search).to_lowercase())
                            {
                                header {
                                    a class="source" href=(&requirement.source.url) target="_blank" rel="noopener noreferrer" data-translatable {
                                        (&requirement.source.title)
                                    }
                                    span class={ "status " (status.css_class()) } { (status.label()) }
                                }
                                blockquote data-translatable { (&requirement.text) }
                                @if let Some(reason) = &requirement.reason {
                                    p class="reason" data-translatable { (reason) }
                                }
                                @if requirement.tests.is_empty() && requirement.scope.expects_tests() {
                                    p class="empty" { "No test ID is assigned." }
                                }
                                @if !requirement.tests.is_empty() {
                                    table {
                                        thead { tr { th { "Result" } th { "Test" } th { "Source" } } }
                                        tbody {
                                            @for id in &requirement.tests {
                                                @if !test_applies_to_platform(id, platform) {
                                                    tr {
                                                        td { span class="test-status other-platform" { "OTHER PLATFORM" } }
                                                        td colspan="2" { code { (id) } }
                                                    }
                                                } @else if let Some(result) = results.get(id) {
                                                    tr {
                                                        td { span class={ "test-status " (result.outcome.css_class()) } { (result.outcome.label()) } }
                                                        td {
                                                            strong data-translatable { (&result.title) }
                                                            details {
                                                                summary { "Test ID" }
                                                                code { (id) }
                                                            }
                                                            @if let Some(failure) = &result.failure {
                                                                pre class="failure" { (failure) }
                                                            }
                                                            @if let Some(reason) = &result.skipped_reason {
                                                                p class="skip-reason" { (reason) }
                                                            }
                                                        }
                                                        td class="test-source" {
                                                            (&result.source.file) ":" (result.source.line)
                                                            br;
                                                            (result.duration_ms) " ms"
                                                        }
                                                    }
                                                } @else {
                                                    tr {
                                                        td { span class="test-status missing-result" { "MISSING RESULT" } }
                                                        td colspan="2" { code { (id) } }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                script { (PreEscaped(SCRIPT)) }
            }
        }
    }
    .into_string()
}

const STYLE: &str = r#"
:root { color-scheme: light dark; font-family: "Segoe UI", "Yu Gothic UI", sans-serif; }
body { margin: 0; background: #f5f7fa; color: #18202a; }
.layout { display: grid; grid-template-columns: 320px minmax(0, 1100px); justify-content: center; gap: 18px; padding: 18px; }
.toc { position: sticky; top: 18px; align-self: start; max-height: calc(100vh - 36px); overflow: auto; border: 1px solid #d8dee6; border-radius: 8px; background: #fff; }
.toc h2 { position: sticky; top: 0; margin: 0; padding: 10px 12px; border-bottom: 1px solid #e6eaf0; background: inherit; font-size: 14px; }
.toc nav { display: grid; padding: 7px; }
.toc a { overflow: hidden; padding: 5px 7px; border-radius: 4px; color: #52606d; font-size: 12px; text-decoration: none; text-overflow: ellipsis; white-space: nowrap; }
.toc a:hover { background: #edf2f7; color: #18202a; }
main { min-width: 0; }
.summary { display: flex; align-items: center; gap: 14px; margin-bottom: 12px; }
h1 { margin: 0 0 4px; font-size: 22px; }
.summary p { margin: 0; color: #52606d; font: 12px/1.5 ui-monospace, monospace; }
.translation { display: flex; min-width: 0; align-items: center; gap: 8px; margin-left: auto; }
.translation-control { display: flex; align-items: center; gap: 6px; color: #52606d; font-size: 12px; font-weight: 600; }
.translation-status { max-width: 320px; color: #66717e; font-size: 11px; white-space: nowrap; }
.translation-retry { margin-left: 3px; padding: 3px 7px; border: 1px solid #aeb8c4; border-radius: 5px; background: #fff; color: #27313d; cursor: pointer; font: inherit; font-weight: 600; }
.translation-retry:hover, .translation-retry:focus-visible { background: #edf2f7; outline: none; }
.toolbar { display: flex; gap: 8px; margin-bottom: 12px; }
input, select { padding: 7px 9px; border: 1px solid #cbd3dc; border-radius: 6px; background: #fff; color: inherit; }
input { flex: 1; }
.requirement { margin-bottom: 10px; overflow: hidden; border: 1px solid #d8dee6; border-radius: 8px; background: #fff; }
.requirement > header { display: flex; align-items: center; gap: 8px; padding: 9px 11px; border-bottom: 1px solid #e6eaf0; }
.source { min-width: 0; flex: 1; overflow: hidden; color: #315b8a; font-size: 13px; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
.status, .test-status { padding: 2px 6px; border-radius: 4px; color: #fff; font-size: 10px; font-weight: 750; white-space: nowrap; }
.covered, .pass { background: #137333; } .uncovered { background: #b58105; } .failed, .missing-result, .fail { background: #b3261e; } .other-platform, .skipped, .skip, .other-component, .out-of-scope, .deferred { background: #6b7280; }
blockquote { margin: 0; padding: 13px 15px; font-size: 15px; line-height: 1.55; }
.reason, .empty { margin: 0; padding: 8px 11px; border-top: 1px solid #e6eaf0; color: #52606d; font-size: 12px; }
.empty { color: #8a5b00; background: #fff8df; }
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th, td { padding: 7px 10px; border-top: 1px solid #e6eaf0; text-align: left; vertical-align: top; }
th { color: #66717e; font-size: 10px; text-transform: uppercase; }
th:first-child, td:first-child { width: 62px; }
.test-source { width: 33%; color: #66717e; font: 10px/1.5 ui-monospace, monospace; overflow-wrap: anywhere; }
details { margin-top: 3px; color: #66717e; font-size: 10px; }
code { overflow-wrap: anywhere; font-family: ui-monospace, monospace; }
.failure { overflow: auto; color: #8c1d18; white-space: pre-wrap; }
.skip-reason { margin: 4px 0 0; color: #66717e; }
[hidden] { display: none !important; }
@media (max-width: 820px) { .layout { display: block; padding: 12px; } .toc { position: static; max-height: 220px; margin-bottom: 12px; } .summary { flex-wrap: wrap; } .translation { width: 100%; margin-left: 0; } }
@media screen and (prefers-color-scheme: dark) {
  body { background: #11161c; color: #edf2f7; }
  .toc, .requirement, input, select, .translation-retry { background: #1a2129; border-color: #35404c; color: #edf2f7; }
  .toc h2, .requirement > header, .reason, .empty, th, td { border-color: #35404c; }
  .toc a, .summary p, .translation-control, .translation-status, .reason, .test-source, details { color: #aeb8c4; }
  .toc a:hover, .translation-retry:hover, .translation-retry:focus-visible { background: #26313c; color: #edf2f7; }
  .source { color: #9cc9ff; }
  .empty { color: #ffe08a; background: #302900; }
  .failure { color: #ffb4ab; }
}
@page { margin: 10mm; }
@media print {
  :root { color-scheme: light; }
  body { background: white; color: black; font-size: 9pt; line-height: 1.2; }
  .layout { display: block; margin: 0; padding: 0; }
  .toc, .toolbar, .translation, .summary a, .test-source, details { display: none !important; }
  .summary { gap: 8pt; margin-bottom: 6pt; }
  h1 { font-size: 14pt; }
  .summary p { color: #333; font-size: 7pt; }
  .requirement { margin-bottom: 4pt; overflow: visible; border: 0; border-top: 1px solid #888; border-radius: 0; background: white; }
  .requirement > header { gap: 5pt; padding: 3pt 0; break-after: avoid; }
  .source { overflow: visible; white-space: normal; overflow-wrap: anywhere; color: black; font-size: 8pt; text-decoration: none; }
  .status, .test-status { padding: 0 2pt; border: 1px solid #777; border-radius: 0; background: white; color: black; font-size: 7pt; }
  blockquote { padding: 3pt 0; font-size: 9pt; line-height: 1.2; orphans: 2; widows: 2; }
  .reason, .empty { padding: 2pt 0; color: black; background: white; font-size: 8pt; }
  table { font-size: 8pt; }
  th, td { padding: 2pt 3pt; overflow-wrap: anywhere; }
  th { font-size: 7pt; }
  th:first-child, td:first-child { width: 65pt; }
  th:last-child { display: none; }
  tr { break-inside: avoid; }
  .failure { overflow: visible; color: black; white-space: pre-wrap; overflow-wrap: anywhere; }
}"#;

const SCRIPT: &str = r#"
const search = document.querySelector('#search');
const status = document.querySelector('#status');
function filter() {
  const query = search.value.toLowerCase();
  for (const item of document.querySelectorAll('.requirement')) {
    item.hidden = !item.dataset.search.includes(query)
      || (status.value !== 'all' && item.dataset.status !== status.value);
  }
}
search.addEventListener('input', filter);
status.addEventListener('change', filter);

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

function restoreRequirement(requirement) {
  for (const element of requirement.querySelectorAll('[data-translatable]')) {
    element.textContent = originalTexts.get(element);
  }
  requirement.dataset.translationLanguage = 'original';
}

async function translateRequirement(requirement, targetLanguage, version) {
  if (requirement.dataset.translationLanguage === targetLanguage) return;
  const elements = [...requirement.querySelectorAll('[data-translatable]')];
  const translations = await Promise.all(
    elements.map(element => translateText(originalTexts.get(element), targetLanguage))
  );
  if (language.value !== targetLanguage || version !== translationVersion) return;
  for (const [index, element] of elements.entries()) {
    element.textContent = translations[index];
  }
  requirement.dataset.translationLanguage = targetLanguage;
}

async function translateReport() {
  const targetLanguage = language.value;
  const cards = [...document.querySelectorAll('.requirement')];
  if (targetLanguage === 'original') {
    for (const card of cards) restoreRequirement(card);
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
        await translateRequirement(nearby || pending[0], targetLanguage, version);
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

const requirementObserver = new IntersectionObserver(() => {
  translateReport();
}, { rootMargin: '75% 0px', threshold: 0.01 });
for (const requirement of document.querySelectorAll('.requirement')) requirementObserver.observe(requirement);
language.addEventListener('change', () => {
  updateLanguageQuery(language.value);
  prepareTranslators(language.value);
  translateReport();
});
if (language.value !== 'original') {
  prepareTranslators(language.value);
  translateReport();
}
"#;

#[cfg(test)]
mod tests {
    use super::super::TestSource;
    use super::*;

    keyringd_testkit::scenario! {
        group "generate" {
            case "Platform reports keep recorded evidence and distinguish other platforms from missing results" {
                verify "retain recorded evidence and label platform exclusions separately from missing runs" {
                    std::thread::spawn(|| {
                    let directory = tempfile::tempdir().unwrap();
                            let results = directory.path().join("linux");
                            fs::create_dir(&results).unwrap();
                            let requirements = directory.path().join("requirements.yaml");
                            fs::write(&requirements, "requirements:\n  - source:\n      title: Endpoint access\n      url: https://example.test/spec#endpoint\n    text: Protect the endpoint.\n    tests:\n      - shared::endpoint\n      - keyringd-windows::endpoint\n  - source:\n      title: Unrecorded test\n      url: https://example.test/spec#missing\n    text: Record shared behavior.\n    tests:\n      - shared::missing\n").unwrap();
                            fs::write(
                                results.join("case.json"),
                                serde_json::json!({
                                    "id": "shared::endpoint", "title": "Endpoint", "outcome": "passed",
                                    "duration_ms": 1, "source": { "file": "C:/another-host/source.rs", "line": 10 },
                                    "source_code": "captured evidence"
                                })
                                .to_string(),
                            )
                            .unwrap();
                            let output = directory.path().join("requirements.html");
                            generate(&ReportPaths {
                                requirements,
                                test_results: results,
                                output: output.clone(),
                            })
                            .unwrap();
                            let html = fs::read_to_string(output).unwrap();
                            assert!(
                                html.contains(
                                    "2 requirements · 1 covered · 0 uncovered · 0 failed · 1 missing results"
                                )
                            );
                            assert!(html.contains("OTHER PLATFORM"));
                            assert!(html.contains("shared::missing"));
                    }).join().unwrap();
                }
            }
        }
    }

    fn ledger() -> Ledger {
        serde_saphyr::from_str(
            r#"
requirements:
  - source:
      title: CTAP 2.2 — Example
      url: https://example.test/ctap.html#example
    text: Platforms MUST do the thing.
    tests:
      - crate::tests::passes
  - source:
      title: CTAP 2.2 — Uncovered
      url: https://example.test/ctap.html#uncovered
    text: Platforms MUST do another thing.
    tests: []
  - source:
      title: CTAP 2.2 — Missing result
      url: https://example.test/ctap.html#missing-result
    text: Platforms MUST do a third thing.
    tests:
      - crate::tests::has_no_current_result
  - source:
      title: CTAP 2.2 — Authenticator requirement
      url: https://example.test/ctap.html#authenticator
    text: Authenticators MUST do the thing.
    scope: other-component
    reason: This behavior belongs to the authenticator.
  - source:
      title: CTAP 2.2 — Unsupported feature
      url: https://example.test/ctap.html#unsupported
    text: Platforms MAY support another feature.
    scope: out-of-scope
    reason: The application does not support this feature.
"#,
        )
        .unwrap()
    }

    keyringd_testkit::scenario! {
        group "render" {
            case "Renders requirement and test result statuses" {
                verify "display the recorded outcome of each requirement and test" {
                    std::thread::spawn(|| {
                        let mut results = BTreeMap::new();
                        results.insert(
                            "crate::tests::passes".to_owned(),
                            TestResult {
                                id: "crate::tests::passes".to_owned(),
                                title: "passes".to_owned(),
                                source: TestSource {
                                    file: "tests/example.rs".to_owned(),
                                    line: 10,
                                },
                                outcome: TestOutcome::Passed,
                                duration_ms: 1,
                                failure: None,
                                skipped_reason: None,
                            },
                        );

                        let html = render(&ledger(), &results, None);

                        assert!(
                            html.contains(
                                "5 requirements · 1 covered · 1 uncovered · 0 failed · 1 missing results"
                            )
                        );
                        assert!(html.contains("Platforms MUST do the thing."));
                        assert!(html.contains("href=\"https://example.test/ctap.html#example\""));
                        assert!(html.contains("crate::tests::passes"));
                        assert!(html.contains("No test ID is assigned."));
                        assert!(html.contains("MISSING RESULT"));
                        assert!(html.contains("value=\"other-component\">OTHER COMPONENT</option>"));
                        assert!(html.contains("value=\"out-of-scope\">OUT OF SCOPE</option>"));
                        assert!(html.contains("crate::tests::has_no_current_result"));
                        assert!(html.contains("id=\"translation-status\""));
                        assert!(html.contains("data-translatable"));
                        assert!(html.contains("Translator.create"));
                        assert!(html.contains("Translation needs confirmation."));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "validate" {
            case "Rejects a non default scope without a reason" {
                verify "reject a custom requirement scope without an explanation" {
                    std::thread::spawn(|| {
                        let ledger: Ledger = serde_saphyr::from_str(
                            r#"
requirements:
  - source:
      title: CTAP 2.2 — Reset
      url: https://example.test/ctap.html#reset
    text: Authenticators MUST reset.
    scope: out-of-scope
"#,
                        )
                        .unwrap();

                        assert!(validate(&ledger).is_err());
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
