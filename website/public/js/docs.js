const mobileDocs = window.matchMedia('(max-width: 700px)');

for (const pre of document.querySelectorAll('.doc-content pre')) {
  const code = pre.querySelector('code');
  const text = (code?.textContent ?? pre.textContent).replace(/\n$/, '');
  const button = document.createElement('button');
  let resetLabel;

  pre.classList.add('copyable-code');
  button.type = 'button';
  button.className = 'copy-code';
  button.textContent = 'Copy';
  button.setAttribute('aria-label', 'Copy code to clipboard');

  button.addEventListener('click', async () => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.setAttribute('readonly', '');
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.append(textarea);
        textarea.select();
        if (!document.execCommand('copy')) throw new Error('Copy command failed');
        textarea.remove();
      }

      window.clearTimeout(resetLabel);
      button.textContent = 'Copied';
      button.setAttribute('aria-label', 'Code copied to clipboard');
      resetLabel = window.setTimeout(() => {
        button.textContent = 'Copy';
        button.setAttribute('aria-label', 'Copy code to clipboard');
      }, 1800);
    } catch {
      button.textContent = 'Select code';
      button.setAttribute('aria-label', 'Copy failed; select the code manually');
    }
  });

  pre.append(button);
}

for (const link of document.querySelectorAll('.docs-nav-links a')) {
  if (new URL(link.href).pathname === window.location.pathname) {
    link.setAttribute('aria-current', 'page');
  }
}

const docsNav = document.querySelector('.docs-nav');
const docsNavToggle = document.querySelector('.docs-nav-toggle');

function setDocsNavOpen(open) {
  if (!docsNav || !docsNavToggle) return;
  docsNav.classList.toggle('is-open', open);
  docsNavToggle.setAttribute('aria-expanded', String(open));
}

if (docsNav && docsNavToggle) {
  docsNavToggle.addEventListener('click', () => {
    setDocsNavOpen(docsNavToggle.getAttribute('aria-expanded') !== 'true');
  });

  docsNav.addEventListener('click', event => {
    if (mobileDocs.matches && event.target.closest('.docs-nav-links a')) {
      setDocsNavOpen(false);
    }
  });

  docsNav.addEventListener('keydown', event => {
    if (event.key === 'Escape' && docsNavToggle.getAttribute('aria-expanded') === 'true') {
      setDocsNavOpen(false);
      docsNavToggle.focus();
    }
  });

  mobileDocs.addEventListener('change', () => setDocsNavOpen(false));
}

const toc = document.querySelector('#toc');
let pageContents;
if (toc) {
  pageContents = document.createElement('details');
  pageContents.id = toc.id;
  pageContents.className = toc.className;

  const title = toc.querySelector('#toctitle');
  const summary = document.createElement('summary');
  summary.id = title.id;
  summary.textContent = title.textContent;
  title.remove();
  pageContents.append(summary, ...toc.childNodes);
  toc.replaceWith(pageContents);
}

function updatePageContents() {
  if (pageContents) pageContents.open = !mobileDocs.matches;
}

updatePageContents();
mobileDocs.addEventListener('change', updatePageContents);

if (pageContents) {
  pageContents.addEventListener('click', event => {
    if (mobileDocs.matches && event.target.closest('a[href]')) {
      pageContents.open = false;
    }
  });
}

const installationPlatforms = {
  windows: document.querySelector('.installation-platform.platform-windows'),
  macos: document.querySelector('.installation-platform.platform-macos'),
  ubuntu: document.querySelector('.installation-platform.platform-ubuntu'),
  fedora: document.querySelector('.installation-platform.platform-fedora'),
  wsl: document.querySelector('.installation-platform.platform-wsl'),
};

if (Object.values(installationPlatforms).every(Boolean)) {
  const platformOrder = ['windows', 'macos', 'ubuntu', 'fedora', 'wsl'];
  const platformLabels = {
    windows: 'Windows',
    macos: 'macOS',
    ubuntu: 'Ubuntu',
    fedora: 'Fedora',
    wsl: 'WSL2',
  };
  const firstPanel = installationPlatforms[platformOrder[0]];
  const tabset = document.createElement('div');
  tabset.className = 'installation-tabset';

  const platformHeading = document.createElement('h2');
  platformHeading.className = 'installation-platform-heading';
  platformHeading.textContent = 'Installation by platform';

  const tablist = document.createElement('div');
  tablist.className = 'installation-tablist';
  tablist.setAttribute('role', 'tablist');
  tablist.setAttribute('aria-label', 'Choose your operating system');

  const panels = document.createElement('div');
  panels.className = 'installation-panels';
  const tabs = new Map();

  firstPanel.before(platformHeading, tabset);
  tabset.append(tablist, panels);

  for (const platform of platformOrder) {
    const panel = installationPlatforms[platform];
    const tab = document.createElement('button');
    tab.type = 'button';
    tab.className = 'installation-tab';
    tab.id = `installation-tab-${platform}`;
    tab.textContent = platformLabels[platform];
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-controls', `installation-panel-${platform}`);
    tab.setAttribute('aria-selected', 'false');
    tab.tabIndex = -1;

    panel.id = `installation-panel-${platform}`;
    panel.dataset.installationPlatform = platform;
    panel.setAttribute('role', 'tabpanel');
    panel.setAttribute('aria-labelledby', tab.id);
    panel.hidden = true;

    tablist.append(tab);
    panels.append(panel);
    tabs.set(platform, tab);
  }

  if (pageContents) {
    const platformHeadingIds = new Set(
      Object.values(installationPlatforms)
        .map(panel => panel.querySelector('h2[id]')?.id)
        .filter(Boolean),
    );
    for (const link of pageContents.querySelectorAll('a[href^="#"]')) {
      if (platformHeadingIds.has(link.hash.slice(1))) link.closest('li')?.remove();
    }
  }

  function platformForHash() {
    const hash = decodeURIComponent(window.location.hash.slice(1));
    if (hash === 'linux') return 'ubuntu';
    if (platformOrder.includes(hash)) return hash;
    const target = hash && document.getElementById(hash);
    if (!target || typeof target.closest !== 'function') return undefined;
    return target.closest('[data-installation-platform]')?.dataset.installationPlatform;
  }

  function selectPlatform(platform, { focus = false, updateHash = false } = {}) {
    if (!installationPlatforms[platform]) return;

    for (const name of platformOrder) {
      const selected = name === platform;
      const tab = tabs.get(name);
      const panel = installationPlatforms[name];
      tab.setAttribute('aria-selected', String(selected));
      tab.tabIndex = selected ? 0 : -1;
      panel.hidden = !selected;
    }

    localStorage.setItem('keyringd-installation-platform', platform);
    if (updateHash) history.replaceState(null, '', `#${platform}`);
    if (focus) tabs.get(platform).focus();
  }

  for (const [platform, tab] of tabs) {
    tab.addEventListener('click', () => selectPlatform(platform, { updateHash: true }));
    tab.addEventListener('keydown', event => {
      const currentIndex = platformOrder.indexOf(platform);
      let nextIndex;
      if (event.key === 'ArrowRight') nextIndex = (currentIndex + 1) % platformOrder.length;
      if (event.key === 'ArrowLeft') nextIndex = (currentIndex - 1 + platformOrder.length) % platformOrder.length;
      if (event.key === 'Home') nextIndex = 0;
      if (event.key === 'End') nextIndex = platformOrder.length - 1;
      if (nextIndex === undefined) return;
      event.preventDefault();
      selectPlatform(platformOrder[nextIndex], { focus: true, updateHash: true });
    });
  }

  const initialPlatform = platformForHash()
    || localStorage.getItem('keyringd-installation-platform')
    || platformOrder[0];
  selectPlatform(platformOrder.includes(initialPlatform) ? initialPlatform : platformOrder[0]);

  window.addEventListener('hashchange', () => {
    const platform = platformForHash();
    if (platform) selectPlatform(platform);
  });
}
