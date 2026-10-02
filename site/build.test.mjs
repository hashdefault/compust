import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, readFile, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build, renderDocument, rewriteLink } from './build.mjs';
import { locales } from './content.mjs';
import { base, origin } from './templates.mjs';

test('documentation links resolve from their original source locations', () => {
  assert.equal(rewriteLink('ROADMAP.pt-BR.md#proximos-passos', 'docs/ARCHITECTURE.pt-BR.md'), '/compust/pt-br/docs/roadmap/#proximos-passos');
  assert.equal(rewriteLink('../README.md', 'docs/ARCHITECTURE.md'), '/compust/docs/getting-started/');
  assert.equal(rewriteLink('LICENSE', 'README.md'), 'https://github.com/hashdefault/compust/blob/main/LICENSE');
  assert.equal(rewriteLink('https://example.org/x11', 'README.md'), 'https://example.org/x11');
});

test('Markdown is inert HTML with unique, linkable headings', () => {
  const { html, headings } = renderDocument('README.md', '# Guide\n\n<script>alert(1)</script>\n\n[unsafe](javascript:alert(1))\n\n## Configuração\n\n## Configuração\n', locales.pt);
  assert.ok(!html.includes('<script>'));
  assert.ok(!html.includes('href="javascript:'));
  assert.deepEqual(headings.map(({ id }) => id), ['configuracao', 'configuracao-1']);
});

test('every published page, asset, locale link, and local fragment resolves', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'compust-site-test-'));
  try {
    const routes = await build(directory);
    assert.equal(routes.length, 10);
    const pages = new Map();
    for (const route of [...routes, `${base}404.html`]) {
      const file = route.endsWith('/') ? `${route.slice(base.length)}index.html` : route.slice(base.length);
      const html = await readFile(join(directory, file), 'utf8');
      pages.set(route, html);
      assert.equal((html.match(/<h1[\s>]/g) || []).length, 1, route);
      assert.ok(html.includes(`lang="${route.includes('/pt-br/') ? 'pt-BR' : 'en-US'}"`), route);
      const ids = [...html.matchAll(/\bid="([^"]+)"/g)].map((match) => match[1]);
      assert.equal(new Set(ids).size, ids.length, `duplicate ID: ${route}`);
    }
    for (const [route, html] of pages) {
      for (const [, href] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
        const url = new URL(href, `${origin}${route}`);
        if (url.origin !== origin) continue;
        assert.ok(url.pathname.startsWith(base), `escaped project base: ${href}`);
        const target = pages.get(url.pathname);
        if (target) {
          if (url.hash) assert.ok(target.includes(`id="${decodeURIComponent(url.hash.slice(1))}"`), `missing fragment: ${route} -> ${href}`);
        } else {
          assert.ok((await stat(join(directory, url.pathname.slice(base.length)))).isFile(), `missing asset: ${href}`);
        }
      }
    }
    assert.ok(!(await readFile(join(directory, 'sitemap.xml'), 'utf8')).includes('404.html'));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
