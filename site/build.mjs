import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, rm, copyFile } from 'node:fs/promises';
import { dirname, posix, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import MarkdownIt from 'markdown-it';
import { locales } from './content.mjs';
import { page, landing, documentPage, missingPage, base, origin, repository, home, guideUrl, escape } from './templates.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const sourceRoutes = new Map(Object.values(locales).flatMap((locale) => locale.guides.map((guide) => [guide.source, guideUrl(locale, guide.slug)])));

export function rewriteLink(href, source) {
  if (/^(?:[a-z][a-z\d+.-]*:|\/\/|#)/i.test(href)) return href;
  const url = new URL(href, `https://source.invalid/${source}`);
  const target = decodeURIComponent(url.pathname.slice(1));
  const route = sourceRoutes.get(target);
  if (route) return `${route}${url.search}${url.hash}`;
  if (href.startsWith('/')) return href;
  return `${repository}/blob/main/${target.split('/').map(encodeURIComponent).join('/')}${url.hash}`;
}

export function renderDocument(source, markdown, locale) {
  const parser = new MarkdownIt({ html: false, linkify: false });
  const headings = [];
  const seen = new Map();
  const tokens = parser.parse(markdown.replace(/^\[English \(US\)\].*$/m, ''), {});
  for (let index = 0; index < tokens.length; index++) {
    const token = tokens[index];
    if (token.type === 'heading_open') {
      const text = tokens[index + 1].content;
      const stem = text.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '').replace(/[^\p{L}\p{N}\s-]/gu, '').trim().replace(/\s+/g, '-') || 'section';
      const count = seen.get(stem) || 0;
      seen.set(stem, count + 1);
      const id = count ? `${stem}-${count}` : stem;
      token.attrSet('id', id);
      if (token.tag === 'h2') headings.push({ id, text });
    }
    for (const child of token.children || []) {
      if (child.type === 'link_open') child.attrSet('href', rewriteLink(child.attrGet('href'), source));
    }
  }
  const fence = parser.renderer.rules.fence;
  parser.renderer.rules.fence = (...args) => fence(...args).replace('<pre>', `<pre tabindex="0" aria-label="${locale.code}">`);
  parser.renderer.rules.table_open = () => `<div class="table-scroll" tabindex="0" role="region" aria-label="${locale.table}"><table>\n`;
  parser.renderer.rules.table_close = () => '</table></div>\n';
  return { html: parser.renderer.render(tokens, parser.options, {}), headings };
}

export async function build(output = resolve(root, 'site/dist')) {
  await rm(output, { recursive: true, force: true });
  await mkdir(resolve(output, 'assets'), { recursive: true });
  const assets = {};
  for (const extension of ['css', 'js']) {
    const content = await readFile(resolve(root, `site/assets/site.${extension}`));
    const digest = createHash('sha256').update(content).digest('hex').slice(0, 12);
    assets[extension] = `site.${digest}.${extension}`;
    await writeFile(resolve(output, 'assets', assets[extension]), content);
  }
  await copyFile(resolve(root, 'site/assets/favicon.svg'), resolve(output, 'assets/favicon.svg'));
  const routes = [];
  async function emit(path, html) {
    const destination = resolve(output, `${path.slice(base.length)}index.html`);
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, html);
    routes.push(path);
  }
  for (const locale of Object.values(locales)) {
    await emit(home(locale), page({ locale, title: locale.headline, description: locale.description, body: landing(locale), assets }));
    for (const guide of locale.guides) {
      const markdown = await readFile(resolve(root, guide.source), 'utf8');
      const { html, headings } = renderDocument(guide.source, markdown, locale);
      await emit(guideUrl(locale, guide.slug), page({ locale, title: guide.title, description: guide.description, body: documentPage(locale, guide, html, headings), slug: guide.slug, assets }));
    }
  }
  await writeFile(resolve(output, '404.html'), page({ locale: locales.en, title: 'Page not found', description: 'Find the Compust project and documentation in English or Brazilian Portuguese.', body: missingPage(), assets, notFound: true }));
  await writeFile(resolve(output, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${routes.map((route) => `<url><loc>${escape(origin + route)}</loc></url>`).join('')}</urlset>\n`);
  await writeFile(resolve(output, '.nojekyll'), '');
  console.log(`Built ${routes.length} pages and a bilingual 404 page in ${posix.relative(root, output)}.`);
  return routes;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await build();
