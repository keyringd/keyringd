const mobileDocs = window.matchMedia('(max-width: 700px)');

for (const link of document.querySelectorAll('.docs-nav-links a')) {
  if (new URL(link.href).pathname === window.location.pathname) {
    link.setAttribute('aria-current', 'page');
  }
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
