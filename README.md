<img src="docs/icon.png" width="128" alt="">

# Defixum

A quiet moment between the urge and the tab.

![The pause screen at dusk: a forest under a low sun, asking "What were you about to do before you opened youtube.com?"](docs/pause-dusk.jpg)

Your hands know the way to YouTube before you've decided to go there. Defixum is a small macOS menu-bar app that steps in at that moment. When you land on a site you've chosen, a forest clearing fills the screen and gives you a few slow breaths and a question. Then the choice is yours: close the tab, or open the site for a few minutes, on purpose.

There's no browser extension to install and nothing to switch off when you're tired. Defixum works with Zen, Firefox, Safari, Chrome, Arc, Brave and Edge.

## A pause, not a wall

Most of the time you don't need to be stopped, only to be asked. The pause is a countdown you can watch as a breath, a ring that fills, or one that drains. While it runs, Defixum asks something worth a second's thought: what you were about to do, what the visit costs you, or how many times you've already been back today.

When it ends, **Close the tab** takes you back to the page you came from, past every site you've paused on the way. If there's nowhere to go back to, the tab closes. **Open it for 2, 5 or 15 minutes** lets you in, and the menu bar shows how long you have left.

![Late at night under the stars, the pause asks you to type "I am destined to do great things." before the site opens](docs/pause-night.jpg)

For the sites that pull hardest, you can make yourself type a sentence before going in. Some are honest: "I'm trading time I will never get back for x.com." Some remind you who you'd rather be: "I am destined to do great things." Either way, by the time you've typed it, you've decided.

## When a pause isn't enough

- **Block** a site completely, with no way through, for when you already know the answer.
- **Daily limits** on how many times you open a site, or how many minutes you spend there. Once you've used them, it waits until tomorrow.
- **Schedules** so a rule only applies when it matters: work hours, weeknights, overnight.
- **Limit ⌘Tab** for the days your attention keeps jumping between apps. After too many switches in a minute, ⌘Tab rests for a while and a small note says when it comes back. Off by default.

## Evenings that end when you meant them to

![A bedtime reminder under the moon: "Bedtime in 30 minutes. You planned to sleep at 23:00."](docs/bedtime.jpg)

Tell Defixum when you'd like to be asleep, and the same forest gently covers the screen 2 hours, 1 hour, 30, 15 and 5 minutes before, and at bedtime. It's a nudge, not a lock: **Got it** puts you back where you were, and **Don't remind again today** lets the night be yours. Off by default.

## Watch the habit loosen

![The Activity view: a week of pauses split into turned back, opened anyway and left without choosing](docs/activity.jpg)

The Activity view shows each day of the past week: how often you were paused, how often you turned back, and how much time went to the sites you'd rather spend less time on. Nothing leaves your Mac.

## Made to feel calm

The pause isn't a warning screen. It's a clearing in a spruce forest, drawn fresh each time. The sky follows your clock, from birds in the morning light through golden hour to a night of stars, or you can keep it always light or always dark. With **Weather** on, rain, snow, fog, storms and cloud match the sky outside your window.

Weather uses an approximate location from your internet address (via ipwho.is, or geojs.io as a fallback) and the forecast from Open-Meteo. You can turn it off in Setup, and then Defixum makes no network requests at all.

![The Rules view: sites, pause or block, countdown style, and the messages shown while you wait](docs/settings.jpg)

## Install

1. Download the `.dmg` from the [latest release](https://github.com/ismayilkarimli/defixum/releases/latest) and drag Defixum into Applications. It works on Apple Silicon and Intel Macs running macOS 13 or later.
2. The app isn't notarized, so macOS blocks it the first time. Open it once, then go to **System Settings → Privacy & Security** and click **Open Anyway**. You can also run:
   ```bash
   xattr -dr com.apple.quarantine /Applications/Defixum.app
   ```
3. Defixum appears in the menu bar. Open **Setup** and grant **Accessibility** access, which is needed for Zen and Firefox. Safari and Chromium browsers ask for permission the first time.

### How it sees the tab you're on

Defixum reads only the address of the tab in front of you. Firefox-based browsers like Zen and Firefox are read through macOS Accessibility. Safari and Chromium browsers (Chrome, Arc, Brave, Edge…) are asked directly through Apple Events. Your rules, history and activity stay in a folder on your Mac.

## Running it locally

**You need:** macOS 13+, the [Rust toolchain](https://rustup.rs), [Node.js](https://nodejs.org) 18+, and the Xcode Command Line Tools (`xcode-select --install`).

```bash
git clone https://github.com/ismayilkarimli/defixum.git
cd defixum
npm install
npm run dev        # run in development mode
npm run build      # build Defixum.app into src-tauri/target/release/bundle/macos
npm run release    # universal (Apple Silicon + Intel) .app and .dmg, ad-hoc signed
```

The settings window opens when you click the menu-bar icon. Set `DEFIXUM_SHOW=1` to open it on launch.

macOS ties Accessibility and Automation permissions to the app's code signature. An ad-hoc signed build loses those permissions every time you rebuild. To keep them, sign with your own certificate:

```bash
APPLE_SIGNING_IDENTITY="Apple Development: Your Name (TEAMID)" npm run build
```

To keep test data separate from your real rules, point the app at a scratch folder with `DEFIXUM_DATA=/some/folder`. Logs go to `defixum.log` in that folder.
