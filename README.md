# Defixum

## What it is

Defixum is a small macOS menu-bar app that makes you stop and breathe before you open a distracting website. When you land on a site you've added (YouTube, Reddit, X…), a calm full-screen pause appears over the browser. Once the countdown ends you can close the tab, or open the site for a few minutes.

It works without any browser extension. Defixum reads the address of the tab you're looking at directly: through Accessibility for Firefox-based browsers like Zen and Firefox, and by asking Safari and Chromium browsers (Chrome, Arc, Brave, Edge…) directly.

- **Pause before websites**: a breathing countdown before the page is usable.
- **Block mode**: no way through, only the option to close the tab.
- **Daily limits**: cap how many times a day you open a site, or how many minutes you spend on it.
- **Schedules**: rules can apply only on certain days and times, including overnight.
- **Activity**: see how often you turned back versus went through, over the last 7 days.
- **Countdowns and messages**: breathe, fill or drain countdowns, and optional messages while you wait: a question to reflect on, what the visit costs you, or a sentence to type out before you can continue.
- **A forest under your sky**: the pause is a clearing in a forest drawn fresh each time. The sky follows your clock, with stars at night and birds by day, or you can fix it to light or dark.
- **The weather outside**: rain, snow, fog, storms and clouds on the pause match your local weather. This uses an approximate location from your internet address (ipwho.is) and the weather from Open-Meteo. Turn it off in Setup if you'd rather not.
- **Menu bar timer**: shows the time left when you've opened a site for a few minutes.

### Install

1. Download the `.dmg` from the [latest release](https://github.com/ismayilkarimli/defixum/releases/latest) and drag Defixum into Applications. The download works on Apple Silicon and Intel Macs running macOS 13 or later.
2. The app isn't notarized, so macOS blocks it the first time. Open it once, then go to **System Settings → Privacy & Security** and click **Open Anyway**. You can also run:
   ```bash
   xattr -dr com.apple.quarantine /Applications/Defixum.app
   ```
3. Defixum appears in the menu bar. Open **Setup** and grant **Accessibility** access, which is needed for Zen and Firefox. Safari and Chromium browsers ask for permission the first time.

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
