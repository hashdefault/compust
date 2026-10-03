# Compust website design

## 0. Research log

The public site introduces the project, explains its maturity, and leads readers to
the same Markdown documentation maintained with the compositor. English (US) and
Brazilian Portuguese have equivalent pages and navigation.

The embedded reference shortlist was Warp, Ollama, and Vercel. The selected pair
was the minimalist style reference and Warp: restrained editorial composition,
warm neutrals, precise terminal surfaces, and readable monospace details. This is
an original Compust design; no third-party logos, illustrations, or code are used.

Lazyweb searches for `developer tools terminal landing page` and `open source
documentation` returned six references. Two screenshots were viewed: Tapcart's
developer tools page and Termius's Linux download page. The useful patterns were
an immediate explanation, one primary documentation action, a substantial product
illustration, and clearly grouped reference links. Their gradients, advertising
claims, and device mockups were not adopted.

The spatial reference is StyleGallery's
[sticky-aside](https://github.com/changeroa/StyleGallery/blob/main/patterns/split-sidebar/sticky-aside.md).
Documentation uses a sticky reading navigator with ordinary document scrolling.
At narrow widths the navigator returns to normal flow above the article.

Two generated desktop concepts compared a split hero with a centered hero. The
split concept was selected: approximately 44% text and 56% illustration, a thin
navigation rule, warm paper, dark overlapping windows, and three unboxed feature
columns. The research image is a local development artifact, not a shipped asset.
Its invented terminal output is replaced with the real example configuration.
The headline says “new layer” because a performance advantage is not established.

Interaction research used the [beUI button source](https://beui.dev/r/button).
The extracted mechanism is immediate feedback on the control itself, interruptible
opacity transitions, and a reduced-motion alternative. Native buttons implement
the demo without React or a motion dependency.

## 1. Identity and content

Compust is a small, experimental Rust compositor for an existing X11 session.
The tone is curious, precise, and welcoming to contributors. Never imply native
Wayland support, production readiness, universal driver support, or a demonstrated
performance advantage over picom. The desktop scene is explicitly an illustration.

The home page follows the reader's decisions: understand the project (hero), see
the effects (interactive illustration and feature row), understand the boundary
and current status (project section), build or contribute (documentation links),
and find upstream references (footer). The guide pages prioritize reading.

## 2. Color tokens

| Token | Value | Use |
| --- | --- | --- |
| paper | `#f5f3ec` | Main canvas |
| ink | `#242523` | Text and primary controls |
| muted | `#5c5e58` | Supporting copy |
| line | `#d1d0c7` | Structural rules |
| surface | `#ebe9e0` | Hover, code, active navigation |
| terminal | `#202320` | Illustration and code surface |
| terminal-raised | `#30352f` | Terminal title bars |
| terminal-ink | `#f5f3ec` | Dark-surface text |
| terminal-muted | `#bdc3b7` | Secondary dark-surface text |
| focus | `#335940` | Keyboard focus outline |

Window transparency uses terminal at 78% alpha. Demo-only wallpaper tones are
`#485047`, `#788375`, and `#a5b29a`. They carry no interface meaning. No gradients.

## 3. Typography

Use the local system sans-serif stack for prose and the system monospace stack
for labels and commands. No font download, layout shift, or third-party request.
The hero is regular-weight, tightly tracked, and scales from 44px to 72px. Page
titles scale from 36px to 48px; section headings are 28px; feature titles are 24px.
Body text is 16px with 1.65 line height; hero copy is 18px. Small labels are 12px,
navigation is 14px, and code is 13px. Prose has a 70ch maximum measure.

## 4. Spacing and layout

Spacing scale: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96px. The page limiter
is 1200px with 24px narrow and 48px wide gutters. The header has at least 80px of
height. Major sections use 80px vertical space, reduced to 48px on narrow screens.
The hero becomes a single column below 960px; documentation becomes a single
column below 800px. Intrinsic grids shrink safely below their preferred width.

The browser document owns vertical scrolling. The header scrolls away normally.
The documentation aside sticks 24px from the top only in the wide layout. Tables
and code may scroll horizontally inside labeled, keyboard-focusable regions.
Table cells retain normal word boundaries so option names and Portuguese headers
remain readable; the table region absorbs overflow instead of splitting identifiers.
No page-level horizontal overflow. Navigation wraps; there is no hidden mobile menu.

## 5. Reusable components

`page` constrains width. `cluster` wraps related navigation and actions. `eyebrow`
is a short monospace label, never a paragraph. `button` has primary and secondary
variants, minimum 44px height, visible focus, and immediate hover/press feedback.
`text-link` uses an underline. `section-heading` separates an index from its title.
`link-row` exposes a heading, a description, and a visible destination affordance.

`desktop-preview` contains a decorative CSS wallpaper and two terminal windows.
Its caption identifies a browser illustration. Three real buttons toggle blur,
transparency, and window visibility; `aria-pressed` reflects each state. Controls
are hidden until JavaScript attaches, while the illustrated scene remains visible
without JavaScript. The terminal content is decorative and hidden from assistive
technology; the figure caption provides the equivalent explanation.

`doc-nav` links every guide and the current article's headings. Current pages
use `aria-current=page` and a tonal fill. `prose` renders semantic Markdown with
stable heading IDs, readable tables, selectable commands, and source-edit links.
The locale switch always preserves the current guide, and uses full language names
for accessible labels. `site-footer` groups project and upstream links.

A local-only primitive showcase verifies typography, controls, focus, active
navigation, link rows, terminal surfaces, and code before composing full pages.

## 6. Interaction and motion

Motion tokens: `quick` 120ms, `fade` 180ms, easing `cubic-bezier(.2,0,.2,1)`.
Only the window visibility demo animates, using opacity; toggling it mid-transition
retargets the same element. Blur and transparency changes apply immediately so
readers can compare states. No autoplay, parallax, cursor tracking, or background
loop. Under reduced motion the visibility transition is removed. No animation
library or client-side routing is needed.

## 7. Surfaces and depth

Corner radius is 4px for buttons, code blocks, and terminal windows; the primary
page remains unboxed. A fine neutral outline defines window edges. The terminal
window shadow is `0 16px 40px #00000040`. The desktop preview has a restrained
inset rule, layered wallpaper, and two offset windows. The effects belong to the
illustration; prose and navigation stay opaque and legible.

## 8. Accessibility, performance, and review

Readers include Linux users evaluating the project, Rust contributors looking for
an entry point, and Portuguese speakers reading on a phone. Each can reach the
appropriate build instructions or contribution guide directly from the home page.

Use semantic landmarks, one main heading, a skip link, visible keyboard focus,
native buttons, meaningful link names, locale metadata, and reduced-motion support.
Avoid meaning conveyed by color alone. Text aims for WCAG AA contrast. Long words,
200% zoom, table overflow, and narrow Portuguese navigation are QA cases.

The production site contains static HTML, shared CSS, and a tiny optional demo
script. No analytics, cookies, framework runtime, remote fonts, or third-party
scripts. JavaScript is unnecessary for reading or navigation. Check all routes at
375, 768, and 1280px, every demo state, language switching, deep links, and 404s.
Review real browser screenshots and run accessibility/performance audits.

Accepted design debt: none. Generated reference wording is not product evidence;
the repository documentation is the authority for feature claims.
