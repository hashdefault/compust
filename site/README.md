# Compust website

[English (US)](README.md) | [Português (Brasil)](README.pt-BR.md)

The public project site is published at <https://hashdefault.github.io/compust/>.
English lives at `/compust/`; Brazilian Portuguese lives at `/compust/pt-br/`.
Both languages include getting started, architecture, roadmap, and contribution
guides. The guides are generated from the repository's Markdown files, so edit
those files instead of generated HTML.

## Work on the site

Use Node.js 24 or newer. From the repository root:

```sh
npm ci --ignore-scripts
npm test
npm run build
npm run preview
```

Open <http://127.0.0.1:4173/compust/>. The preview serves `site/dist/`, including
the project path used on GitHub Pages. Rebuild after editing; there is no live
reload. `PORT=4174 npm run preview` selects another port. Stop with Ctrl+C.

`content.mjs` contains the bilingual home-page text. `templates.mjs` defines the
shared navigation, home page, documentation layout, and 404 page. `build.mjs`
converts Markdown, resolves source-relative documentation links, fingerprints
assets, and emits the sitemap. `assets/` contains the shared style, optional
illustration controls, and favicon. The only build dependency is `markdown-it`;
no dependencies or third-party requests are needed in the browser.

Follow [DESIGN.md](../DESIGN.md) when changing the UI. The local-only component
showcase can be previewed with `SITE_DIR=site npm run preview` at
<http://127.0.0.1:4173/compust/primitives.html>. It is excluded from the build.

The test suite checks the generated link graph, locale paths, heading fragments,
asset existence, and safe Markdown handling. Also inspect both languages at
375, 768, and 1280px, navigate with a keyboard, try all three illustration
controls, check reduced motion, and confirm the docs work with JavaScript disabled.

## Publish

In the repository's **Settings → Pages → Build and deployment**, select **GitHub
Actions** as the source. This is a one-time repository setting; the workflow's
`GITHUB_TOKEN` cannot enable Pages itself. The **Website** workflow tests and
builds pull requests, then deploys successful pushes to `main`. It can also be
started manually from the Actions tab. Generated files stay out of Git.

The URL is a project site, so links and assets intentionally include `/compust/`.
If the repository moves or gains a custom domain, update `base` and `origin` in
`templates.mjs` and the preview base in `serve.mjs`, then check the generated
canonical URLs, locale alternates, sitemap, and links before publishing.

The browser scene illustrates effects using CSS. It is not a hardware benchmark,
a real Compust capture, or a claim that Compust runs inside the browser. Keep that
distinction in both translations when changing the illustration or its copy.
