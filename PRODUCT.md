# Product

<!-- impeccable:product-schema 1 -->

## Platform

web (Tauri 2 webviews on macOS: a settings window and a full-screen pause overlay; vanilla HTML/CSS/JS in `ui/`)

## Stack

Vanilla HTML, CSS and JS served by Tauri, talking to Rust through `window.__TAURI__`. No framework or build step for the UI.

## Users

The general public on macOS: people who want to scroll less and don't think of themselves as technical. They meet Defixum in two moments:

- **The pause.** They just opened YouTube, Reddit or X on autopilot, often tired or late at night. A full-screen pause covers the browser. They are mid-impulse, a little annoyed, and need to calm down enough to make an actual choice.
- **The settings window.** Calm, deliberate moments when they decide which sites to pause, how hard to make it, and look back at how often they turned away.

First launch has to make sense without a manual, including granting macOS permissions.

## Purpose

Put a moment of friction and calm between impulse and a distracting website, so the person can close the tab instead of drifting in. Success is the tab closed. A deliberate, time-boxed visit is the second-best outcome.

## Mechanism

Works with no browser extension. It reads the active tab's address through Accessibility (Zen, Firefox) or by asking Safari and Chromium browsers directly, then covers the browser with its own overlay.

## Capabilities

- Rules: a named group of sites, each paused or blocked.
- Pause: a countdown styled as breathe, fill or drain. After it, close the tab or open the site for one of a few preset minute choices. "Close the tab" goes back in the tab's history to the last page no rule holds (past every paused or blocked page on the way), and closes the tab if there is none.
- Messages while waiting: Reflect (mindful questions), Count the cost (time and sleep tradeoffs, some only at night), Type to unlock (copy a sentence word for word: either a pointed guilt sentence or an encouraging one, such as "I am destined to do great things.").
- Block mode, daily open and minute limits, day and time schedules.
- Activity: turned back versus opened over 7 days, per-rule counts and time today.
- Weather (on by default, can be turned off): the pause sky shows the weather outside, using an approximate IP location and Open-Meteo.
- Bedtime reminder (off by default): set a sleep time and a calm full-screen reminder covers whatever is in front 2 hours, 1 hour, 30, 15 and 5 minutes before it, at bedtime, then every 15 minutes until 2 hours past it. Each has "Got it" and "Don't remind again today". Reminders wait while the display is asleep, and one left up for 3 hours after bedtime is taken down on its own.
- Limit ⌘Tab (off by default): after more than N app switches in a minute (6 unless changed), ⌘Tab is swallowed for a cooldown (60 seconds unless changed); each refused press shows a small toast under the menu bar for 5 seconds.
- Menu bar: the logo with time left on the soonest-expiring unlock; the menu lists every open unlock.

## Voice

Plain, calm and direct. It doesn't preach, moralize or sound clinical. The guilt sentences are deliberately pointed, but the UI around them stays gentle.

## Brand commitments

- Name: Defixum. There is a menu-bar logo; there is no other brand system.
- Feeling (user-stated): natural and outdoorsy, reducing anxiety and inducing calm. Not preachy, not clinical.
- World (user-pinned): a forest. Spruce tree lines in depth, fog between them, and a sky that follows the clock and the weather outside.

## Accessibility

Respect reduced motion (the countdown animations must have a still alternative), visible keyboard focus, and both light and dark system appearance.

## Open decisions

- No pricing or monetization; distributed free on GitHub releases.
