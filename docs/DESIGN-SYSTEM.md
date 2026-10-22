# Design system

The rules every stylesheet in `src/styles` follows. `npm run lint:styles`
checks them (see [Checks](#checks)).

## Where things go

| File | Holds |
|---|---|
| `base.css` | Every token with its default (light) value, the default dark values, resets. |
| `themes/*.css` | Token overrides per theme and mode. A theme may also style a few components for its signature look (receipt edges, terminal prompts) — keep those short and use tokens there too. Colors appear only as token values, never inside those component rules. |
| `themes/_native.css` | Rules shared by Mica, Acrylic and macOS. The leading underscore loads it before their own files. |
| `components/*.css` | How components look, written only in tokens. No colors, no theme names. |
| `platform-*.css` | What the operating system dictates (fonts, window corners, scrollbars). |

A component never names a theme (`.theme-paper .x`). If a theme needs a
component to look different, prefer a component token (`--card-radius`,
`--tag-chip-font-family`, `--layout-item-background`, …): give it a default in
`base.css` and set it in the theme. Only a look no token can express goes in a
rule in the theme's file.

## Tokens

Two levels:

1. **Base values**: colors, radii, type sizes, durations, layers.
2. **Component tokens** built from them: `--button-*`, `--card-*`,
   `--input-*`, `--menu-*`, `--switch-*`, … Themes mostly override these.

### Color roles

| Token | Use | Minimum contrast on `--bg-content`, `--bg-element`, `--bg-panel`, `--bg-input` |
|---|---|---|
| `--text-primary` | Content, labels | 7:1 |
| `--text-secondary` | Descriptions, source app, notes | 5.5:1 |
| `--text-muted` | Times, counts, placeholders, section labels | 4.5:1 |
| `--accent-color` / `--on-accent` | Primary actions and the text on them | 4.5:1 for `--on-accent` on the accent |
| `--status-success/-warning/-danger/-info` (+ `-rgb`) | State | — |

Dim text by choosing a lower role, never with `opacity`: opacity compounds
with an already muted color and fell to 1.7:1 for the item time. Disabled
controls are the exception (`opacity: 0.45`).

Translucent colors come from RGB-triplet tokens so a theme can retint them:
`rgba(var(--shadow-rgb), 0.08)` for shadows, `--neutral-rgb` for hairlines that
must work on any surface, `--highlight-rgb` for light edges on dark surfaces,
`--accent-color-rgb` and `--status-*-rgb` for tints. Fixed colors that the
theme must not change have their own names: `--scrim`, `--media-backdrop`,
`--on-media`, `--qr-backdrop`, `--window-close-background`, `--traffic-*`,
`--file-tint-*`.

### Type

`--font-size-2xs` (10px) to `--font-size-2xl` (20px). 10px is the smallest
readable size in the compact window. Relative units (`em`, `%`) are fine for
text that scales with its container.

### Spacing

Paddings, margins and gaps use **0, 1, 2, 4, 6, 8, 10, 12, 16, 20, 24 px**,
then any multiple of 4. The half-steps (2, 6, 10) exist for the compact
window; 1px is for hairline offsets. Values from tokens or `calc()` on tokens
are not checked.

### Radius

Components use tokens: `--radius-2xs` (2) `--radius-xs` (4) `--radius-sm` (6)
`--radius-md` (8) `--radius-lg` (12), `--radius-pill`, or a component token
(`--card-radius`, `--button-radius`, `--input-radius`, …). Nested corners may
offset a token: `calc(var(--card-radius) - 4px)`. `0`, `50%` and `inherit`
are fine.

### Motion

| Token | Value | For |
|---|---|---|
| `--duration-fast` | 120ms | Hover and press feedback |
| `--duration-base` | 180ms | Opening, selecting, revealing |
| `--duration-slow` | 240ms | Larger surfaces |

Animate `opacity` and `transform`. Do not animate layout (`width`, `height`,
grid tracks, `top`/`left`): when the page structure itself moves, the user
loses their place. Reduced motion sets all three durations to 0. Looping
animations (spinners, pulses) may use their own duration.

### Layers

`--z-popover` (1000) < `--z-dropdown` < `--z-overlay` < `--z-context-menu` <
`--z-modal` < `--z-toast` < `--z-texture` (9000). Plain numbers below 1000 are
stacking local to a component (a hovered row above its neighbours).

## Overrides

No `!important`. When a rule loses, fix the cause instead:

- a theme styling a component directly instead of setting a token;
- a selector more specific than it needs to be (`:last-child` counts like a
  class, so `.a .b > div:last-child` beats `.a.c .b`);
- generic rules written after specific ones in the same file;
- a style set inline from a script (move it to CSS);
- several states fighting over one property: let them set a custom property
  and apply it in one rule (see `--compact-preview-reserve`).

Exceptions carry `/* design-ok: reason */`, either after a declaration or in a
rule's header to cover the whole rule. Current ones: the transparent window
resets of the compact preview and pin sticker windows (they must beat any
theme, including installed packages) and `prefers-reduced-motion`.

The app supports macOS 10.13, whose WebKit has no cascade layers (`@layer`),
so precedence is managed with selectors, not layers.

## Checks

```bash
npm run lint:styles             # fails if any file breaks more rules than before
npm run lint:styles -- --report # every violation, with file and line
npm run lint:styles -- --update # after fixing some, lower the baseline
```

Rules checked: `no-raw-color`, `no-important`, `no-theme-in-component`,
`font-size-token`, `radius-token`, `motion-token`, `spacing-scale`,
`z-index-token`, and token hygiene (`unused-token`, `undefined-token`).

`scripts/style-lint-baseline.json` holds known violations per file and rule;
counts may only go down. It is empty: the stylesheets currently follow every
rule. A declaration the rules cannot express gets `/* design-ok: reason */`.

`src/styles/contrast.test.ts` resolves each theme's tokens in light and dark
mode and fails when a text role drops below its minimum contrast.
