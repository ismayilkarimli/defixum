---
name: Defixum
description: A clearing at the edge of a spruce forest, under the sky you would see right now.
colors:
  # settings window, light appearance (ui/style.css :root)
  mist-ground: "#e9ede7"
  lichen-panel: "#e0e7df"
  birch-surface: "#f8faf6"
  spruce-ink: "#14231c"
  bark-muted: "#52635a"
  fog-faint: "#66766d"
  field-line: "#87948b"
  hairline: "#cdd6cd"
  moss: "#2d6a48"
  moss-ink: "#ffffff"
  moss-soft: "#d5e4d5"
  resin-amber: "#93590f"
  resin-amber-soft: "#f1e3c8"
  opened-grey: "#74877b"
  rust-danger: "#b0432e"
  strip-sky-top: "#b2c8c3"
  strip-sky-bottom: "#dfe7df"
  strip-tree-far: "#9cb3a7"
  strip-tree-near: "#23402f"
  # settings window, dark theme ([data-theme="dark"], chosen by the Sky setting)
  mist-ground-dark: "#0f1815"
  lichen-panel-dark: "#0b1310"
  birch-surface-dark: "#16221d"
  spruce-ink-dark: "#e1e9de"
  bark-muted-dark: "#93a398"
  fog-faint-dark: "#7a8a80"
  field-line-dark: "#607066"
  hairline-dark: "#22312a"
  moss-dark: "#8dc29c"
  moss-ink-dark: "#0b1a12"
  moss-soft-dark: "#1d3527"
  resin-amber-dark: "#e3ac5d"
  resin-amber-soft-dark: "#32281a"
  opened-grey-dark: "#5f7366"
  rust-danger-dark: "#e57b62"
  strip-sky-top-dark: "#09151a"
  strip-sky-bottom-dark: "#2a4741"
  strip-tree-far-dark: "#1d332d"
  strip-tree-near-dark: "#040b09"
  # pause overlay: the sky over the trees, anchor stops of the clock (ui/forest.js SKY)
  night-sky-top: "#07131a"
  night-horizon: "#22403c"
  day-sky-top: "#a5c0bf"
  day-horizon: "#dfe8de"
  golden-sky-top: "#c4ad80"
  golden-horizon: "#e8c48a"
  dusk-sky-top: "#20364a"
  dusk-horizon: "#5c6e69"
  near-spruce-day: "#0a1c15"
  near-spruce-night: "#020806"
  starlight: "#eef3e6"
  # pause overlay: words in the sky (ui/overlay.html :root and :root.night)
  sky-ink-day: "#10221a"
  site-moss-day: "#2a6644"
  ring-mark-day: "#2b5a42"
  sky-amber-day: "#8a520f"
  sky-ink-night: "#e8f0e5"
  site-lichen-night: "#cfe2a4"
  ring-mark-night: "#e3ecd7"
  sky-amber-night: "#e8b866"
  # pause overlay: the sun (day) and moon (night) the countdown sits on
  sun-hi: "#fffef7"
  sun: "#f5f4e7"
  sun-edge: "#e4e9da"
  moon-hi: "#f1f5e8"
  moon: "#dbe3cf"
  moon-edge: "#c3cfb9"
  low-sun-hi: "#fbe3b4"
  low-sun: "#ecb873"
  low-sun-edge: "#d99a50"
  disc-ink: "#13251c"
  # pause overlay: the forest floor, always dark
  floor-ink: "#e6eee3"
  lichen: "#dbe7c9"
  lichen-lit: "#edf4e1"
  lichen-ink: "#0e1e16"
typography:
  display:
    fontFamily: "Optima, \"Optima Nova\", Candara, \"Segoe UI\", sans-serif"
    fontSize: "36px"
    fontWeight: 400
    lineHeight: 1.18
  numeral:
    fontFamily: "Optima, \"Optima Nova\", Candara, \"Segoe UI\", sans-serif"
    fontSize: "64px"
    fontWeight: 400
    lineHeight: 1
    fontFeature: "\"tnum\""
  headline:
    fontFamily: "Optima, \"Optima Nova\", Candara, \"Segoe UI\", sans-serif"
    fontSize: "30px"
    fontWeight: 400
    lineHeight: 1.2
  title:
    fontFamily: "Optima, \"Optima Nova\", Candara, \"Segoe UI\", sans-serif"
    fontSize: "22px"
    fontWeight: 400
    lineHeight: 1.35
  aside:
    fontFamily: "Optima, \"Optima Nova\", Candara, \"Segoe UI\", sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.45
  body:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  body-pause:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.5
  caption:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
rounded:
  xs: "6px"
  sm: "7px"
  md: "8px"
  lg: "10px"
  xl: "12px"
  field: "14px"
  pill: "999px"
spacing:
  xs: "6px"
  sm: "10px"
  md: "16px"
  lg: "22px"
  xl: "32px"
  pause-gap: "40px"
  gutter: "44px"
components:
  tab-segment:
    textColor: "{colors.bark-muted}"
    rounded: "{rounded.sm}"
    padding: "4px 16px"
    typography: "{typography.body}"
  tab-segment-selected:
    backgroundColor: "{colors.birch-surface}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.sm}"
    padding: "4px 16px"
  rule-row:
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.md}"
    padding: "9px 10px"
  rule-row-current:
    backgroundColor: "{colors.birch-surface}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.md}"
    padding: "9px 10px"
  sheet:
    backgroundColor: "{colors.birch-surface}"
    rounded: "{rounded.xl}"
    padding: "14px 16px"
  choice-option:
    backgroundColor: "{colors.birch-surface}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.xl}"
    padding: "13px 15px"
  choice-option-selected:
    backgroundColor: "{colors.moss-soft}"
    textColor: "{colors.spruce-ink}"
    padding: "13px 15px"
  site-chip:
    backgroundColor: "{colors.moss-soft}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.xs}"
    padding: "2px 4px 2px 9px"
  text-input:
    backgroundColor: "{colors.birch-surface}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.sm}"
    padding: "4px 8px"
  button-quiet:
    backgroundColor: "{colors.mist-ground}"
    textColor: "{colors.spruce-ink}"
    rounded: "{rounded.sm}"
    padding: "4px 12px"
  button-quiet-hover:
    backgroundColor: "{colors.mist-ground}"
    textColor: "{colors.moss}"
  day-toggle:
    backgroundColor: "{colors.birch-surface}"
    textColor: "{colors.bark-muted}"
    rounded: "{rounded.md}"
    width: "42px"
    padding: "6px 0"
  day-toggle-on:
    backgroundColor: "{colors.moss}"
    textColor: "{colors.moss-ink}"
    rounded: "{rounded.md}"
    width: "42px"
  switch-on:
    backgroundColor: "{colors.moss}"
    rounded: "{rounded.pill}"
    width: "38px"
    height: "22px"
  pause-close:
    backgroundColor: "{colors.lichen}"
    textColor: "{colors.lichen-ink}"
    rounded: "{rounded.pill}"
    padding: "0.7rem 1.4rem"
  pause-close-hover:
    backgroundColor: "{colors.lichen-lit}"
    textColor: "{colors.lichen-ink}"
  pause-minutes:
    textColor: "{colors.floor-ink}"
    rounded: "{rounded.pill}"
    padding: "0.5rem 0.95rem"
  answer-field:
    textColor: "{colors.sky-ink-day}"
    rounded: "{rounded.field}"
    padding: "0.75rem 1rem"
---

# Design System: Defixum

## Overview

**Creative North Star: "The Clearing at the Forest Edge"**

Defixum lives at the edge of a spruce forest. The pause overlay is the clearing itself: five procedural tree lines stand in depth from a far, fog-softened ridge to near-black giants that frame the screen, fog pools at the foot of each distant line, and the sky above them is the sky of the current hour, interpolated from the clock so the light drifts from mist to honey to stars instead of switching themes. The countdown is the sun by day and the moon by night; the question hangs in the open sky; the way out is one lit lichen button on the dark forest floor.

The settings window is the same forest seen from the trailhead and quieted for everyday use. A thin strip of live sky and a two-depth tree line crowns the window; everything beneath it is mist-grey ground and one hairline-ruled sheet, with moss green marking what is on and chosen. Rules are marked with a small spruce, filled when the rule is on and drawn in outline when it is off. The density is calm and deliberate: one rule fits on screen as a ruled form, not a stack of padded cards.

The voice is Optima at regular weight for everything that speaks (headlines, rule names, the countdown numerals, the typed sentence, the closing line of the rule list); the macOS system font carries every control. Motion is slow and atmospheric: tree lines rise into place, fog drifts over minutes, the disc breathes on an eight-second cycle, and all of it has a still alternative under reduced motion.

**Key Characteristics:**
- Procedural spruce tree lines, a new stretch of forest on every pause, as the only figurative imagery.
- A sky driven continuously by the clock in the pause; system light/dark appearance in settings.
- Depth by atmosphere (tone mixed from horizon to near spruce, plus fog), not by shadows.
- Moss for on, chosen and turned back; resin amber for attention and over-limit.
- One hairline-ruled sheet per group instead of stacked cards.
- Optima at weight 400 for voice; system UI for controls.

## Colors

A forest palette of mist greys, spruce greens and lichen, warmed by a single resin amber and lit by a sky that changes with the hour.

### Primary
- **Moss** (`{colors.moss}` light / `{colors.moss-dark}` dark): the one active colour in settings. Enabled spruce markers, the checked switch, the selected day toggles, focus outlines, the caret, the turned-back bars in the Activity chart, the "Allowed" status, and the hover state of quiet buttons and the New rule button. Text on a solid moss fill uses Moss Ink.
- **Moss Soft** (`{colors.moss-soft}` / `{colors.moss-soft-dark}`): the tint of a chosen option in a ruled choice frame, the site chips, and text selection.
- **Site Moss / Site Lichen** (`{colors.site-moss-day}` by day, `{colors.site-lichen-night}` at night): in the pause, the site name picked out inside the headline and the typed sentence, the accepted answer-field border, and the ready unlock hint.

### Secondary
- **Resin Amber** (`{colors.resin-amber}` / `{colors.resin-amber-dark}`; `{colors.sky-amber-day}` / `{colors.sky-amber-night}` in the pause): attention without alarm. The dot on the Setup tab, over-limit counts in the Activity table and pause detail, and the "bad" browser status. **Resin Amber Soft** tints the Accessibility row when permission is still missing.
- **Low Sun** (`{colors.low-sun-hi}`, `{colors.low-sun}`, `{colors.low-sun-edge}`): the disc turns into a low amber sun when a site is blocked or a limit is spent.

### Tertiary
- **Lichen** (`{colors.lichen}`, hover `{colors.lichen-lit}`, text `{colors.lichen-ink}`): only the primary "Close the tab" button and the pause focus ring. It is the single lit object on the forest floor.
- **Rust** (`{colors.rust-danger}` / `{colors.rust-danger-dark}`): the Delete rule link, which fills solid when armed for a second click.

### Neutral
- **Mist Ground** (`{colors.mist-ground}` / `{colors.mist-ground-dark}`): window background.
- **Lichen Panel** (`{colors.lichen-panel}` / `{colors.lichen-panel-dark}`): the rule list sidebar, a half-step off the ground.
- **Birch Surface** (`{colors.birch-surface}` / `{colors.birch-surface-dark}`): ruled sheets, inputs, the selected tab and the current rule row.
- **Spruce Ink** (`{colors.spruce-ink}` / `{colors.spruce-ink-dark}`): primary text. **Bark Muted** for secondary text, help and labels; **Fog Faint** for placeholders and outlined (off) spruce markers.
- **Hairline** (`{colors.hairline}` / `{colors.hairline-dark}`): every border and rule line, and the empty track of a chart bar.
- **Opened Grey** (`{colors.opened-grey}` / `{colors.opened-grey-dark}`): the "opened anyway" share of a chart bar. Deliberately neutral, not red.
- **Title-bar strip**: sky gradient from `{colors.strip-sky-top}` to `{colors.strip-sky-bottom}` behind a far tree line in `{colors.strip-tree-far}` and a near line in `{colors.strip-tree-near}` (each with a dark counterpart).
- **Floor Ink** (`{colors.floor-ink}`): text on the pause's dark floor; minute chips are Floor Ink at 10% (19% on hover), secondary floor text at 62%.

### The pause sky
The sky is a vertical gradient from a top colour to a horizon colour (horizon reached at 68% of the height). Both are interpolated linearly between these stops by hour of day, in `ui/forest.js`:

| Hour | Top | Horizon |
|---|---|---|
| 0:00 | night-sky-top | night-horizon |
| 4:30 | #0b1b22 | #2a4542 |
| 6:30 | #8aa3a9 | #d6dccf |
| 9:30 | day-sky-top | day-horizon |
| 16:00 | #a0bcb9 | #dce5d8 |
| 17:24 | #a9b7a4 | #e3d6a6 |
| 18:30 | golden-sky-top | golden-horizon |
| 20:15 | dusk-sky-top | dusk-horizon |
| 22:00 to 24:00 | night-sky-top | night-horizon |

The nearest spruce is mixed between `{colors.near-spruce-night}` and `{colors.near-spruce-day}` by the sky's luminance, and the page background behind everything is that near colour. Each of the five tree lines is filled with the horizon mixed toward the near spruce at tone 0.28, 0.48, 0.67, 0.86 and 1. When the luminance 60% of the way down the sky (where the words sit) drops to 0.2 or below, the point where dark and light ink read equally well, the overlay switches its word and disc colours to the night set (moon instead of sun); stars in `{colors.starlight}` fade in as luminance falls below 0.08.

### Life in the sky

The sky carries a little life, chosen by its light and drawn fresh for each pause. Anything that moves is hidden or held still under reduced motion.

- **Day and golden hour:** small flocks of gull-shaped birds cross the upper sky. Each flock is 2 to 9 birds in a loose V or a scatter, stroked in the horizon mixed toward near spruce. Every bird flaps and glides on its own beat, and the first flock is already midway across when the pause appears. Zero to four soft clouds drift through, tinted by the horizon, so they turn warm at sunset.
- **Golden hour:** warm motes of light rise among the near trees.
- **Night:** stars, about a third of them twinkling, gather along a faint tilted Milky Way. A shooting star crosses every 9 to 35 seconds, and fireflies blink among the third tree line, mostly at the sides of the clearing.

### Weather

With the Weather setting on (`settings.weather`, default on), the pause shows the weather outside. Every 20 minutes the app finds an approximate location by IP (ipwho.is, then get.geojs.io, then the time zone's main city from the system zone table), rounds it to two decimals and asks Open-Meteo for the current WMO code, cloud cover, wind and today's sunrise and sunset. None of these services needs a key. A reading older than 3 hours is dropped, so offline the sky goes back to fair weather. Weather applies under every Sky choice; the real sunrise and sunset only apply under "Time of day".

- **Cloud:** the cover sets how many clouds there are (up to about 11), how large and dense they are, and above 55% a cloud deck pressing down from the top. Gloom drains the sky's colour toward a grey of the same brightness and dims it. It also mutes the sun's glow, golden light and the stars, which are gone under a full sky.
- **Rain and drizzle:** pale streaks on one canvas in front of the trees, each at its own depth and slanted by the wind. They are fainter at night. Far tree lines sink into the haze.
- **Snow:** flakes that sway and drift with the wind, and a paler sky instead of a darker one.
- **Storm:** heavy rain and a darker sky. Lightning lights the sky behind the trees every 6 to 22 seconds as two soft pulses, peaking at an opacity of 0.36 or less and never more than three flashes a second.
- **Fog:** the sky flattens into one pale wash, and the far tree lines nearly disappear (their tones are raised to the power of 3.2).
- **What stays away:** no birds in rain, snow, storms or thick fog. No fireflies in rain or snow. No shooting stars under heavy cloud. No golden motes under gloom.
- **Sun times:** the clock is stretched so the real sunrise lands on 6:36 and the real sunset on 18:36 of the SKY stops. A Tallinn October dusk then arrives at its real time instead of at a fixed hour.
- **Settings window:** in "Time of day", the title-bar sky takes the same cloud tint and sun times.

Preview with `overlay.html?demo=pause&weather=clear|cloudy|overcast|fog|drizzle|rain|downpour|storm|snow&wind=30`. Text contrast follows automatically, because the ink and haze are worked out from the weather-tinted sky.

### Legibility over a moving sky

The muted words on the pause are solid colours: #24362c by day and #cfdacb by night, never ink at reduced opacity. A soft radial haze sits behind the words: light (rgba 250,252,244) by day and dark (rgba 4,12,10) by night. Its strength is 0.08, rising to 0.48 near the dawn and dusk swap, which keeps the detail line at 4.5:1 or better at every hour. Edges of things you type into use `field-line`, 3:1 against the surface; hairlines between rows stay `line`. Light text on dark grounds uses `-webkit-font-smoothing: antialiased`, and dark text keeps the system smoothing.

### Named Rules
**The Clock Is the Theme Rule.** Neither window reads system appearance; the Sky setting (`settings.sky`: `time`, `light` or `dark`) decides. With "Time of day" (the default) the pause takes its palette from the local hour in the Mac's time zone, interpolated between SKY stops and repainted every minute, and the settings window turns dark when the sky is night and tints its title bar with the same sky. "Light" fixes the pause at 11:00 and the settings window light; "Dark" fixes the pause at 23:30 and the settings window dark.

**The Moss Means On Rule.** Moss marks what is enabled, chosen or turned back; amber marks what needs attention or is over its limit. Never use moss for a warning or amber for success, and never show "opened anyway" in a warning colour.

**The Dark Floor Rule.** The forest floor at the bottom of the pause is always the near-spruce colour, day or night. Lichen is the only lit fill on it; everything else there is translucent Floor Ink.

## Typography

**Display Font:** Optima (with "Optima Nova", Candara, "Segoe UI", sans-serif)
**Body Font:** the macOS system font (-apple-system, system-ui, sans-serif)

**Character:** Optima's flared, calligraphic sans gives the words a humane, carved-sign quality without the formality of a serif; the system font keeps controls native and invisible.

### Hierarchy
- **Display** (400, 36px, 1.18): the pause headline or question, balanced, max 20em, with the site name in Site Moss. Drops to 29px under 720px of height.
- **Numeral** (400, 64px, 1, tabular figures): the countdown on the sun or moon, always in Disc Ink. 52px while a sentence is being typed on short screens.
- **Headline** (400, 30px, 1.2): settings page titles (Activity, Setup) and the editable rule name.
- **Title** (400, 22px, 1.35): the sentence to copy in Type to unlock, wrapped in curly quotes.
- **Aside** (400, 15px, 1.45): the "today" line at the foot of the rule list; the empty-editor message uses the same face at 19px.
- **Body** (400, 14px, 1.5): settings text and controls. Help text 12.5px, max 62ch; Activity lede 15px, max 60ch.
- **Body Pause** (400, 15px, 1.5): the pause detail line, max 32rem, and the floor.
- **Label** (600, 13px): section headings within a rule (Sites, When you visit, Countdown) and sidebar group labels in Bark Muted. Tab labels are 500.
- **Caption** (400, 12px): sublines under option names, chart numbers and day letters, table headers (600).

### Named Rules
**The Optima Voice Rule.** Optima is for words that speak to the person: headlines, rule names, numerals, the sentence to type and the quiet closing line. Every control, label, chip and table is the system font.

**The Regular Weight Rule.** Optima is always weight 400. Hierarchy in Optima comes from size alone; weight (500, 600) is reserved for the system font.

**The Tabular Count Rule.** Every number that changes or is compared (countdown, chart counts, table, minute chips, number inputs, slider output) uses tabular figures.

## Layout

**Settings window.** A fixed 64px title bar (sky strip, centred segmented tabs, tree line 30px tall along its bottom edge, drag region) above a full-height content area. The Rules tab is a two-column grid: a 260px rule list on Lichen Panel with a hairline right border, and an editor with 30px top and 44px side padding and a 640px max content width. Activity and Setup use a centred 760px column with 38px/44px/52px padding. Within the editor, each field is separated by a hairline top border with 20px above and 22px below its content; settings rows are 14px by 16px.

**Bedtime screen.** The pause's layout without the typing box and minute chips: disc, headline and detail centred over open sky in the same single 40px gap, the floor holding two actions (lit pill, then chip) and a hint line.

**Pause overlay.** Full-bleed and fixed. The floor band takes `clamp(170px, 21vh, 230px)` at the bottom; the sky region above it centres a single column (ring, headline, sentence, answer box, detail) with one visible gap of 40px between every pair of blocks (32px when the screen is 860px tall or less). The disc box is 300px (220px under 720px of height; 200px while typing on short screens).

### Named Rules
**The One Gap Rule.** Stacked blocks in the pause are separated by exactly one gap, measured between what is visible: text blocks give back their half-leading with negative margins and the disc gives back the empty part of its box. The unlock hint floats inside the gap so filling it moves nothing; minute choices keep their room while invisible during the countdown.

**The Clearing Rule.** Tree lines dip toward the centre of the screen (the near layers fall to 20% to 70% of their height within a centred half-width) so the disc and words always sit over open sky. The title-bar tree line does the same under the tabs.

## Elevation & Depth

Depth is atmospheric, not structural. The pause builds distance from five tree lines whose fills are the horizon colour mixed toward the near spruce, with fog bands (radial-gradient clouds over a faint horizon band at 70% and 38% of the horizon colour) layered between the far lines. On entry each line rises from below in sequence. Settings is flat tonal layering: ground, a half-step-darker panel, and birch surfaces bordered by hairlines; the only shadows are small lifts on the selected tab, the current rule row and the switch knob.

### Shadow Vocabulary
- **Segment lift** (`box-shadow: 0 1px 3px rgba(10, 30, 20, 0.16)`): the selected tab in the title bar.
- **Row lift** (`box-shadow: 0 1px 2px rgba(10, 30, 20, 0.08)`): the current rule in the rule list.
- **Knob** (`box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25)`): the switch thumb.
- **Field ring** (`box-shadow: 0 0 0 1px` moss): focused inputs and the chip field, together with a moss border.
- **Floor lift** (`box-shadow: 0 6px 18px rgba(0, 0, 0, 0.28)`): the Close the tab button on the dark floor.
- **Sun glow** (`box-shadow: 0 0 70px, 0 0 18px` in the disc glow colour): the sun or moon, plus a radial halo behind it.
- **Text lift** (`text-shadow: 0 0 22px rgba(236, 242, 232, 0.55)` by day, `0 1px 14px rgba(0, 8, 6, 0.45)` at night): words in the sky, so they hold against the tree tips.

### Named Rules
**The Atmosphere, Not Shadow Rule.** Distance is made with tone and fog. Sheets, panels and choice frames carry no drop shadow; a shadow appears only on a selected segment, a control's knob, or the single lit button on the floor.

## Shapes

Gently rounded rectangles for everything framed (6px chips and focus outlines, 7px inputs, inner tabs and quiet buttons, 8px rule rows and day toggles, 10px tab track and chip field, 12px sheets and choice frames, 14px pause answer field), full pills for switches, the saved toast and all pause buttons, and circles for the sun, the moon and the progress ring. Chart bars are rounded 7px on top and 2px at the base. Borders are always 1px hairlines; the New rule button is the one dashed border.

The only figurative silhouette is the spruce: drooping tiers alternating an outer branch tip with an inner notch, narrowing to a spire with a slight lean, generated per tree from a seed drawn fresh for each pause (and each settings-window load), so the forest is never the same twice. The same silhouette, reduced to an 11 by 14px path, is the rule marker.

### Named Rules
**The Ruled Sheet Rule.** Related options share one frame split by hairlines (side by side, in thirds, or in rows) and the chosen one is tinted Moss Soft. Settings groups are one bordered sheet with hairline rows, never separate cards.

**The Spruce Silhouette Rule.** Trees are procedural spruces from `Forest.treeline`, seeded fresh for every pause, never images or generic triangles. A rule's on/off state is a filled or outlined spruce.

## Components

### Segmented tabs
Quiet glass over the sky strip.
- **Track:** Birch Surface at 55% with a 1px Spruce Ink 10% border, 10px radius, 3px padding, 2px gaps.
- **Tab:** transparent, Bark Muted, weight 500, 4px by 16px, 7px radius; hover to Spruce Ink.
- **Selected:** Birch Surface fill, Spruce Ink, segment lift. The Setup tab can carry a 7px Resin Amber dot.

### Rule list rows
- **Row:** 8px radius, 9px by 10px, a grid of the spruce marker and the name (600, ellipsis) over a Bark Muted 12px subline.
- **Hover:** Birch Surface at 60%. **Current:** solid Birch Surface with row lift.
- **Off:** outlined spruce in Fog Faint and a muted name.
- **New rule:** dashed Spruce Ink 22% border, 8px radius, Bark Muted; moss text and border on hover.

### Ruled choice frame (signature)
- **Frame:** Birch Surface, 1px hairline, 12px radius, clipped; options in two columns, three columns (with a 40px moss countdown hint drawing above each option) or stacked rows, divided by hairlines.
- **Option:** 13px by 15px, native radio or checkbox in moss.
- **Hover:** Moss Soft at 40%. **Chosen:** Moss Soft. **Focus:** 2px moss outline inset by 2px.

### Site chips
- **Field:** Birch Surface, hairline border, 10px radius, 7px padding, 6px gaps; focus-within adds a moss border and field ring.
- **Chip:** Moss Soft, 6px radius, 13px text, with an 8px stroked SVG cross in Bark Muted that darkens on hover.

### Inputs / Fields
- **Style:** Birch Surface, 1px hairline, 7px radius, 4px by 8px; number inputs 68px wide with tabular figures.
- **Focus:** moss border plus field ring, no outline. **Disabled:** 45% opacity.
- **Rule name:** borderless Optima 30px with a transparent bottom hairline that shows on hover and turns moss on focus.

### Buttons
- **Quiet button:** Mist Ground fill, hairline border, 7px radius, 4px by 12px, 13px text; hover turns border and text moss.
- **Day toggles:** 42px wide, Birch Surface, hairline, 8px radius; pressed is solid moss with Moss Ink.
- **Delete rule:** a borderless rust text link that underlines on hover; the first click arms it into a solid rust 7px-radius button for four seconds.

### Switch
- **Shape:** 38 by 22px pill, Spruce Ink 18% track, 18px white knob with knob shadow.
- **On:** moss track, knob slides 16px over 150ms.

### Activity chart
Seven columns, 14px apart, up to 190px tall. Each bar (max 46px wide, 7px/2px corners) stacks turned-back in moss, opened in Opened Grey and left-without-choosing in Spruce Ink 14%, on a hairline track. Today's day letter is Spruce Ink at 600. A legend of 10px rounded keys sits below; the per-rule table uses hairline rows and amber for over-limit values.

### Countdown disc (signature)
- **Sun / moon:** a radial gradient from the disc highlight to its edge, sun glow, and a radial halo; Disc Ink numerals and a 14px cue (Breathe in / Breathe out) sit on it.
- **Breathe:** core scales 0.62 to 1 and halo 0.72 to 1.12 over 8s ease-in-out; the fog lifts 2.2vh on the in-breath.
- **Fill / Drain:** a 1.4px stroked ring at 64% of the box, starting at 12 o'clock, in the ring mark colour over a 22% track; fill grows clockwise, drain retracts to 12.
- **Settled:** the disc rests at 0.8 scale and the ring dims to 55%. **Blocked:** a low amber sun with no numerals.
- **Reduced motion:** no breathing or drift; the disc holds at 0.8 and the ring steps once per second.

### Bedtime screen
The bedtime reminder is the pause's forest in a second full-screen window (`overlay.html?mode=bedtime`), so the sky, weather, life and tree lines are the same code. It appears over every app, one window level above the pause, and returns the person to the app they were in when it goes.
- **Disc:** the sun or moon at rest at full size (no breathing), showing the time left in a 64px numeral over a 14px cue: `2` hours, then `60`, `30`, `15` and `5` minutes, and `now` at bedtime. It follows the real clock every 30 seconds; a reminder left on screen for more than its five minutes stops saying its own mark and shows the minutes actually left.
- **Words:** the display headline ("Bedtime in 2 hours.", "Bedtime in 1 hour.", "Bedtime in 30 minutes.", "It's bedtime.") and one muted detail line: the planned time plus at most one gentle suggestion that fits the moment ("A good time to start winding down.", "Maybe put the screens away soon.", "Good night."). No exclamation marks, no guilt.
- **Floor:** the lichen "Got it" pill (Esc and Return do the same) and one translucent chip, "Don't remind again today", with the footer hint "Press Esc to dismiss". The minute chips, answer field and unlock hint of the pause are not shown.
- **Settings row:** Setup, General, "Bedtime reminder": a switch and a time field (Birch Surface, `field-line` border, tabular figures, 45% when the switch is off), 12px apart.
- **Schedule:** off by default. Each reminder is due for five minutes from its moment; one that was missed by more than that is skipped, never replayed. Switching it on or moving the time treats the moments already past tonight as done, so turning it on at 22:10 for 23:00 doesn't open with the one-hour reminder.
- **Reduced motion:** nothing extra moves; the entry animation, birds, drift and rain follow the same rules as the pause.

### Pause floor actions
- **Close the tab:** Lichen pill, Lichen Ink text at 600, 0.7rem by 1.4rem, floor lift; hover Lichen Lit, press scales to 0.97, focus is a 2px lichen outline offset 3px.
- **Minute chips:** translucent Floor Ink pills (10%, 19% hover), 14px tabular text, after the muted "or open it for"; disabled at 35% until the answer is accepted.
- **Footer hint:** 13px Floor Ink at 62%; an error replaces it rather than adding a line.

### Answer field (Type to unlock)
- **Style:** centred 16px text, white at 50% by day (7% at night) with a dark 20% (white 17%) hairline, 14px radius, 0.75rem by 1rem; sized up front for the whole sentence so it never grows.
- **Accepted:** Site Moss border with a 1px inset ring. **Focus:** 2px ring-mark outline offset 3px.
- **Hint:** floats in the gap below; turns Site Moss when ready and nudges 4px side to side when Return is pressed too early.

## Do's and Don'ts

### Do:
- **Do** derive every pause colour from the hour through `Forest.sky()` and the tone mix, and keep the repaint each minute.
- **Do** draw trees with `Forest.treeline` and the pause's `sceneSeed`, so every pause is a new stretch of forest and a resize redraws the same one.
- **Do** keep centred text inside the clearing, over open sky, with the text lift shadow.
- **Do** use moss (`{colors.moss}`) for on, chosen and turned back, and resin amber (`{colors.resin-amber}`) for attention and over-limit.
- **Do** group related settings in one hairline-ruled sheet with 12px corners and tint the chosen option Moss Soft.
- **Do** set Optima at weight 400 and size its hierarchy from 15px to 64px; use the system font for controls.
- **Do** keep one 40px visible gap between stacked pause blocks and reserve room for anything that appears later.
- **Do** give every animation a still alternative under `prefers-reduced-motion`.

### Don't:
- **Don't** tie either window to system appearance; the Sky setting drives it, and by default that means the clock.
- **Don't** use tree images, clip-art or plain triangles; the spruce is procedural.
- **Don't** put a light surface or a second lit button on the forest floor.
- **Don't** stack padded cards or add drop shadows to sheets, panels or choice frames.
- **Don't** bold Optima.
- **Don't** colour "opened anyway" as an error; it is a neutral grey.
- **Don't** let tall trees rise behind the disc or the words.
