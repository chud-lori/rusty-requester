# DESIGN.md

Design direction for the Rusty Requester GitHub Pages site
(`docs/index.html`, `docs/usage.html`).

**This file was derived, not authored.** Every field below was read out of
decisions the repository had already made (the app's theme module, the
previous landing page, the icon, the readme) and is recorded here with the
source line. Nothing in it was invented. The `Dial` line and anything under
**Open** need the owner's confirmation before they count as settled.

---

## Identity

Rusty Requester is a native, offline API client: a single binary written in
Rust on `egui`, positioned as a Postman alternative that does not carry a
browser runtime.
Source: `Cargo.toml:8` (crate description), `readme.md:9`.

The name is a deliberate double meaning, and it is the strongest identity
signal the repo contains: Rust the language, and rust as in old hardware that
still works. The stated first audience is developers on older or low-spec
machines.
Source: `readme.md:59-62`.

The mark is a hammer striking a plate, in cream on a rust-red plinth, with
three impact sparks. Cream is `#F4E8D4`, the plinth gradient runs `#E85A40` to
`#B8341F`, and the shadow tone is `#8C2A18` on `#1A1210`.
Source: `docs/icon.svg:5-8, 30-49`.

## Personality

Dry, technical, and willing to state its own limits. The readme documents the
threat model, then names what the user still owns (SSRF from their own
machine, plaintext local data), and the Rust section ends on an "Honest
caveat" that a compromised upstream crate would still bite.
Source: `readme.md:101-105, 141-147`.

The code comments carry the same voice: they record why a decision was made
and what was tried and reverted, in plain sentences.
Source: `src/main.rs:3442-3448` (the reverted macOS title bar note),
`src/theme.rs:22-27` (why the accent was softened twice).

Copy for the site follows that voice: say what the thing does, show the
command, name the limit. No superlatives, because the repo never uses any.

## Palette

The site's palette is taken from the application's own theme module, so the
page and the app read as one product. Two core surfaces plus one accent.

Dark (the app's default theme, `src/model.rs:593-599`):

| Token | Value | Role | Source |
|---|---|---|---|
| canvas | `#121316` | page background | `src/theme.rs:11` |
| surface | `#1B1D22` | cards, panels | `src/theme.rs:12` |
| raised | `#272B32` | inputs, hover | `src/theme.rs:13` |
| border | `#404652` | dividers | `src/theme.rs:14` |
| text | `#F4F7FB` | body text | `src/theme.rs:21` |
| muted | `#A4ACB9` | secondary text | `src/theme.rs:20` |
| accent | `#E25C3E` | one interactive moment | `src/theme.rs:28` |

Light (the app's "Paper Light" theme):

| Token | Value | Role | Source |
|---|---|---|---|
| canvas | `#F1F3F6` | page background | `src/theme.rs:207` |
| surface | `#FFFFFF` | cards, panels | `src/theme.rs:208` |
| raised | `#F6F8FB` | inputs, hover | `src/theme.rs:209` |
| border | `#DADFE6` | dividers | `src/theme.rs:210` |
| text | `#1F2937` | body text | `src/theme.rs:211` |
| muted | `#5C6472` | secondary text | deviation, see below |
| accent | `#C43C28` | one interactive moment | `src/theme.rs:29` |

One documented deviation: the app's light muted is `#6B7280`
(`src/theme.rs:212`), which measures 4.35:1 on the `#F1F3F6` canvas and misses
WCAG AA for normal text. The site darkens it to `#5C6472` (5.37:1). The app
gets away with `#6B7280` because it paints muted text on white panels rather
than on the canvas; a web page cannot rely on that.

The accent is warm rust in both themes and exists because of the product name.
The repo records that it was tuned twice: away from the saturated `#CE422B`
that bled on dark, and away from a coral `#EF5350` that overshot into pink.
Source: `src/theme.rs:22-29`.

Two accent values are deliberately **not** carried onto the site: the per
method colors (`src/theme.rs:15-19`) and the Postman blue
(`src/theme.rs:34`). Both belong to the app's request UI, and importing them
would push the page past three colors with nothing to spend them on.

## Typography

System stacks, no web font, no network request for type. The previous page
already made this choice and the site keeps it.

- Sans: `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Inter",
  "Helvetica Neue", Arial, sans-serif`
- Mono: `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas,
  "Liberation Mono", monospace`

Source: `docs/index.html:32-33` (previous version).

The reason fits the product: the app ships as a single binary with no bundled
runtime, so its documentation should not pull a font file off a CDN to render
a paragraph. Monospace is reserved for things the reader will actually type or
see on screen (commands, paths, key names, setting identifiers), never used as
a display face.

## Mood

Workshop, not showroom. A reference you keep open in a tab while you set the
tool up, closer to a man page with good typography than to a product launch.
Calm surfaces, generous reading measure, the accent spent once per screen.

This follows from what the site has to do: `docs/usage.html` is already the
deep guide with eighteen sections (`docs/usage.html`, section ids `overview`
through `troubleshooting`), so the landing page is the front door and the
quick reference, not a brochure.

## Dial

```
Dial: ENERGY 1 / RHYTHM 2 / MOTION 1
```

Derived, awaiting the owner's confirmation.

- **ENERGY 1.** The product's pitch is restraint: fewer megabytes, no account,
  no telemetry (`readme.md:64-68`). A loud page would argue against the
  product.
- **RHYTHM 2.** Consistent, with real breaks where the content changes kind.
  The page has to hold running prose, a copyable command, and several dense
  reference tables, and those cannot share one composition. It is not a 3,
  because nothing in the repo suggests asymmetry for its own sake.
- **MOTION 1.** Hover and focus feedback plus instant state changes, nothing
  that runs on a timer. A page about cold-start speed and low resource use
  should not animate while the reader is trying to read it.

## Constraints

- Single self-contained HTML file per page, inline CSS and JS, no external
  dependency and no CDN. This is how `docs/index.html` and `docs/usage.html`
  are already built, and GitHub Pages serves them with `.nojekyll`
  (`docs/.nojekyll`).
- Light and dark must both be complete. The app ships three themes and lets
  the user switch (`src/ui/settings.rs:96-102`), so the site shipping a
  half-finished theme would contradict the product.
- Every number on the page must be traceable to a file in this repo or to a
  release artifact. See **Open**, item 1.
- Text contrast meets WCAG AA: 4.5:1 normal, 3:1 large.
- No horizontal overflow at phone width; interactive targets at least 44px.
- No em dash characters in page copy.

## Open

Questions this file could not answer from the repository. They are recorded
rather than guessed.

1. **Are there real measurements for binary size, idle RAM, and cold start?**
   The previous page and the readme both print `~15 MB`, `~30 MB idle RAM` and
   `<100 ms cold start` (`docs/index.html:263-266`, `readme.md:12`), but no
   benchmark, script, or note in the repo produces those figures. They have
   been removed from the rewritten page. If they were measured, where, on what
   hardware, and with what method, so the page can cite it?

2. **Where did the Postman / Insomnia / Bruno comparison figures come from?**
   The download sizes, idle RAM ranges, cold-start ranges and npm dependency
   counts (`docs/index.html:297-304`, `readme.md:72-82`) have no source in the
   repo. The comparison table has been removed. If there is a measurement
   record or a citable article, the table can come back with the citation.

3. **Accent value for the site.** The previous page used `#d97a4e`
   (`docs/index.html:28`), which matches neither theme in `src/theme.rs`. The
   rewrite aligns the site to the app's real accents (`#E25C3E` dark,
   `#C43C28` light). Confirm that alignment is wanted, or say which value is
   canonical.

4. **Default theme for the site.** The app defaults to Dark
   (`src/model.rs:597`) and the site currently opens dark, then follows the
   visitor's system preference and their saved choice. Confirm.

5. **Is a brand typeface intended?** Nothing in the repo names one. The site
   uses system stacks by default. If a face is intended, name it and say
   whether self-hosting it is acceptable given the no-CDN constraint.

6. **Is a Windows build planned?** The source carries
   `#[cfg(target_os = "windows")]` branches (`src/main.rs:3265`,
   `src/ui/settings.rs:336`) and the readme lists a Windows data path
   (`readme.md:276`), but `install.sh:63-67` refuses any OS other than macOS
   and Linux, and `.github/workflows/release.yml` builds only a macOS
   universal DMG and a Linux x86_64 tarball. The page now says exactly that.
   Confirm whether Windows should be described as unsupported or as planned.

7. **Is `SKILL.md` at the repo root meant to be public?** It is not linked
   from any page and its audience is unclear, so the rewrite does not link it.
