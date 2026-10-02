import { locales } from './content.mjs';

export const repository = 'https://github.com/hashdefault/compust';
export const base = '/compust/';
export const origin = 'https://hashdefault.github.io';
export const home = (locale) => `${base}${locale.prefix}`;
export const guideUrl = (locale, slug) => `${home(locale)}docs/${slug}/`;
export const escape = (value) => String(value).replace(/[&<>"']/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const arrow = '<span aria-hidden="true">↗</span>';
const mark = '<svg aria-hidden="true" width="28" height="28" viewBox="0 0 28 28" fill="none"><path d="M2 2h18v18H2z" stroke="currentColor" stroke-width="2"/><path d="M8 8h18v18H8z" fill="currentColor"/><path d="M12 12h10v2H12zm0 5h7v2h-7z" fill="var(--paper)"/></svg>';

function languageLinks(locale, slug) {
  return Object.values(locales).map((entry) => {
    const url = slug ? guideUrl(entry, slug) : home(entry);
    const label = entry.lang === 'en-US' ? 'EN' : 'PT-BR';
    return `<a href="${url}" lang="${entry.lang}" hreflang="${entry.lang}" aria-label="${label}: ${entry.name}"${entry === locale ? ' aria-current="true"' : ''}>${label}</a>`;
  }).join('<span class="separator" aria-hidden="true">/</span>');
}

function header(locale, slug) {
  return `<a class="skip-link" href="#main">${locale.skip}</a>
  <header class="page site-header">
    <a class="brand" href="${home(locale)}" aria-label="Compust">${mark}Compust</a>
    <nav class="site-nav" aria-label="${locale.navigation}">
      <a href="${guideUrl(locale, 'getting-started')}">${locale.docs}</a>
      <a href="${guideUrl(locale, 'roadmap')}">${locale.roadmap}</a>
      <a href="${guideUrl(locale, 'contributing')}">${locale.contribute}</a>
    </nav>
    <nav class="locale" aria-label="${locale.languages}">${languageLinks(locale, slug)}</nav>
  </header>`;
}

function footer(locale) {
  return `<footer class="page site-footer">
    <div class="footer-about"><a class="brand" href="${home(locale)}">Compust</a><p>${locale.footerDescription}</p><p><a href="${repository}/blob/main/LICENSE">${locale.license}</a></p></div>
    <div class="footer-links"><h2 class="eyebrow">${locale.project}</h2><a href="${repository}">${locale.source}</a><a href="${repository}/issues">${locale.issues}</a><a href="${guideUrl(locale, 'contributing')}">${locale.contribute}</a></div>
    <div class="footer-links"><h2 class="eyebrow">${locale.references}</h2><a href="https://www.x.org/wiki/">X.Org</a><a href="https://github.com/X11Libre/xserver">XLibre</a><a href="https://github.com/yshui/picom">picom</a><a href="https://docs.rs/x11rb/0.13.2/x11rb/">x11rb</a></div>
  </footer>`;
}

export function page({ locale, title, description, body, slug, assets, notFound = false }) {
  const path = slug ? guideUrl(locale, slug) : home(locale);
  const alternatives = Object.values(locales).map((entry) => `<link rel="alternate" hreflang="${entry.lang}" href="${origin}${slug ? guideUrl(entry, slug) : home(entry)}">`).join('\n');
  return `<!doctype html>
<html lang="${locale.lang}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <meta name="description" content="${escape(description)}">
  <meta name="theme-color" content="#f5f3ec">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'">
  <title>${escape(title)} · Compust</title>
  ${notFound ? '<meta name="robots" content="noindex">' : `<link rel="canonical" href="${origin}${path}">${alternatives}<link rel="alternate" hreflang="x-default" href="${origin}${slug ? guideUrl(locales.en, slug) : home(locales.en)}">`}
  <meta property="og:type" content="website">
  <meta property="og:title" content="${escape(title)} · Compust">
  <meta property="og:description" content="${escape(description)}">
  <meta property="og:locale" content="${locale.lang.replace('-', '_')}">
  <meta property="og:url" content="${origin}${notFound ? `${base}404.html` : path}">
  <meta name="twitter:card" content="summary">
  <link rel="icon" href="${base}assets/favicon.svg" type="image/svg+xml">
  <link rel="stylesheet" href="${base}assets/${assets.css}">
  ${!slug && !notFound ? `<script src="${base}assets/${assets.js}" defer></script>` : ''}
</head>
<body>${header(locale, slug)}${body}${footer(locale)}</body>
</html>`;
}

function desktop(locale) {
  return `<figure class="desktop-preview">
    <div class="desktop" id="desktop" aria-hidden="true">
      <div class="desktop-bar"><span>compust</span><span>X11 / Rust</span></div>
      <div class="wallpaper"><span></span><span></span><span></span></div>
      <div class="window window-back"><div class="window-title"><span>~/compust</span><span class="dots">○ ○ ○</span></div><pre><span class="terminal-prompt">$</span> cargo build --release

  compust
  X11 compositor in Rust
  v0.1.0 / experimental</pre></div>
      <div class="window window-front"><div class="window-title"><span>compust.toml</span><span class="dots">○ ○ ○</span></div><pre>opacity = 90
fade_ms = 180
blur_radius = 4
max_fps = 120
vsync = true</pre></div>
    </div>
    <div class="demo-controls" role="group" aria-label="${locale.demoLabel}" hidden>
      <button type="button" data-effect="no-blur" aria-pressed="true" aria-controls="desktop">${locale.blur}</button>
      <button type="button" data-effect="no-transparency" aria-pressed="true" aria-controls="desktop">${locale.transparency}</button>
      <button type="button" data-effect="window-hidden" aria-pressed="true" aria-controls="desktop">${locale.window}</button>
    </div>
    <figcaption>${locale.demoCaption}<noscript> ${locale.noScript}</noscript></figcaption>
  </figure>`;
}

export function landing(locale) {
  return `<main id="main" class="page" tabindex="-1">
    <section class="hero" aria-labelledby="hero-title">
      <div><p class="eyebrow">${locale.experimental}</p><h1 id="hero-title">${locale.headline}</h1><p class="intro">${locale.intro}</p><div class="cluster actions"><a class="button" href="${guideUrl(locale, 'getting-started')}">${locale.readDocs}<span aria-hidden="true">→</span></a><a class="text-link" href="${repository}">${locale.github} ${arrow}</a></div></div>
      ${desktop(locale)}
    </section>
    <section class="features" aria-label="${locale.demoLabel}">${locale.features.map(([title, description], i) => `<article><p class="eyebrow">0${i + 1}</p><h2>${title}</h2><p>${description}</p></article>`).join('')}</section>
    <section class="section" aria-labelledby="idea-title"><div class="section-heading"><p class="eyebrow">${locale.ideaLabel}</p><h2 id="idea-title">${locale.ideaHeading}</h2></div><div class="project-copy"><div><p>${locale.idea}</p><p>${locale.inspiration}</p><p><strong>${locale.scope}</strong></p></div><div class="status-note"><strong>${locale.statusTitle}</strong><p>${locale.status}</p><a class="text-link" href="${guideUrl(locale, 'roadmap')}">${locale.statusLink} ${arrow}</a></div></div></section>
    <section class="section" aria-labelledby="docs-title"><div class="section-heading"><p class="eyebrow">${locale.learnLabel}</p><h2 id="docs-title">${locale.learnHeading}</h2></div><div class="link-grid">${locale.guides.map((guide) => `<a class="link-row" href="${guideUrl(locale, guide.slug)}"><div><h3>${guide.title}</h3><p>${guide.description}</p></div>${arrow}</a>`).join('')}</div></section>
    <section class="contribute" aria-labelledby="contribute-title"><div><h2 id="contribute-title">${locale.contributionHeading}</h2><p>${locale.contribution}</p></div><a class="button secondary" href="${guideUrl(locale, 'contributing')}">${locale.contributionAction} ${arrow}</a></section>
  </main>`;
}

export function documentPage(locale, guide, html, headings) {
  return `<div class="page docs-layout"><aside class="doc-nav"><p class="eyebrow">${locale.documentation}</p><nav aria-label="${locale.documentation}">${locale.guides.map((item) => `<a href="${guideUrl(locale, item.slug)}"${item.slug === guide.slug ? ' aria-current="page"' : ''}>${item.title}</a>`).join('')}</nav><div class="toc"><p class="eyebrow">${locale.onPage}</p><nav aria-label="${locale.onPage}">${headings.map(({ id, text }) => `<a href="#${escape(id)}">${escape(text)}</a>`).join('')}</nav></div></aside><main id="main" class="prose" tabindex="-1">${html}<a class="source-link" href="${repository}/edit/main/${guide.source}">${locale.edit} ${arrow}</a></main></div>`;
}

export function missingPage() {
  return `<main id="main" class="page not-found stack" tabindex="-1"><p class="eyebrow">404</p><h1>This page is missing.</h1><p>The address may have changed. Find the project and its documentation below.</p><p lang="pt-BR">Esta página não foi encontrada. Acesse o projeto e a documentação pelos links abaixo.</p><div class="cluster"><a class="button" href="${home(locales.en)}">Compust in English</a><a class="button secondary" href="${home(locales.pt)}" lang="pt-BR">Compust em português</a></div></main>`;
}
