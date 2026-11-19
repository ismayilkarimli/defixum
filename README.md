<p align="center"><img src="docs/icon.png" width="128" alt=""></p>

<h1 align="center">Defixum</h1>

<p align="center">A breath between the urge and the tab.</p>

![The pause screen at dusk, asking "What were you about to do before you opened youtube.com?"](docs/pause-dusk.jpg)

Your hands find YouTube before you decide to go. Defixum catches that moment.

No extension. Works with Zen, Firefox, Safari, Chrome, Arc, Brave and Edge.

## Pause, then choose

A few slow breaths and one honest question. Then close the tab, keep it for later, or go in for 2, 5 or 15 minutes, on purpose.

Closing takes you back past every page you'd rather not be on.

![Under the stars, typing "I am destined to do great things." before the site opens](docs/pause-night.jpg)

For the sites that pull hardest, type a sentence first. By the time you've typed it, you've decided.

## When a pause isn't enough

- **Block** for when you already know the answer.
- **Daily limits** on visits or minutes.
- **Schedules** for work hours, weeknights, late nights.
- **Limit ⌘Tab** when your attention keeps jumping between apps. Hold it 5 seconds when you really need to switch.
- **Remind you to move** after 45 minutes at the screen. Take 1, 3, 5 or 10 minutes, or wave it off. During a video or call it's just a note.

## Sleep on time

![Bedtime reminder under the moon: "Bedtime in 30 minutes."](docs/bedtime.jpg)

Gentle reminders before bed, and again if you are still up: back in 15 minutes, 30, 45, or an hour, your call. Each night can have its own time, or none.

## See it working

![A week of pauses: turned back, opened anyway](docs/activity.jpg)

Every day you turn back, it shows. Nothing leaves your Mac.

## A forest, not a warning

The sky follows your clock and, if you like, your local weather. Turn weather off and Defixum never touches the network.

![Rules: sites, pause or block, countdown, messages](docs/settings.jpg)

## Install

1. Get the `.dmg` from the [latest release](https://github.com/ismayilkarimli/defixum/releases/latest). macOS 13+, Apple Silicon or Intel.
2. It isn't notarized: open it once, then **System Settings → Privacy & Security → Open Anyway**. Or:
   ```bash
   xattr -dr com.apple.quarantine /Applications/Defixum.app
   ```
3. In **Setup**, grant **Accessibility** (for Zen and Firefox). Safari and Chromium browsers ask on first use.

Defixum reads only the address of the tab in front of you. Everything stays on your Mac.

## Running it locally

**You need:** macOS 13+, the [Rust toolchain](https://rustup.rs) and the Xcode Command Line Tools (`xcode-select --install`).

```bash
git clone https://github.com/ismayilkarimli/defixum.git
cd defixum
rustup target add wasm32-unknown-unknown
cargo install trunk tauri-cli
cd src-tauri
cargo tauri dev                       # run in development mode
cargo tauri build --bundles app       # build Defixum.app into src-tauri/target/release/bundle/macos
```

The whole app is Rust. The windows are a Rust → WebAssembly front end (`ui/`, built by [Trunk](https://trunkrs.dev)); the forest drawing is shared with the app through `forest/`.

For a universal (Apple Silicon + Intel) .app and .dmg, ad-hoc signed:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
CI=true APPLE_SIGNING_IDENTITY=- cargo tauri build --target universal-apple-darwin --bundles app,dmg
```

The settings window opens when you click the menu-bar icon. Set `DEFIXUM_SHOW=1` to open it on launch.

macOS ties Accessibility and Automation permissions to the app's code signature. An ad-hoc signed build loses those permissions every time you rebuild. To keep them, sign with your own certificate:

```bash
APPLE_SIGNING_IDENTITY="Apple Development: Your Name (TEAMID)" cargo tauri build --bundles app
```

To keep test data separate from your real rules, point the app at a scratch folder with `DEFIXUM_DATA=/some/folder`. Logs go to `defixum.log` in that folder.
