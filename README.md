<h1 align="center">Discord Quest Completer</h1>

<p align="center">
  Complete Discord Quests without installing the games.<br />
  A small Windows desktop app that runs a tiny stand-in for the game so Discord counts it as played.
</p>

<p align="center">
  <a href="https://github.com/cmxau/discord-quest-completer/actions/workflows/ci.yml"><img src="https://github.com/cmxau/discord-quest-completer/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/cmxau/discord-quest-completer/releases"><img src="https://img.shields.io/github/v/release/cmxau/discord-quest-completer?include_prereleases" alt="Latest release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license" /></a>
  <img src="https://img.shields.io/badge/platform-Windows-0078D4" alt="Windows" />
</p>

> [!NOTE]
> This project builds on
> [markterence/discord-quest-completer](https://github.com/markterence/discord-quest-completer) by
> Mark Terence Tiglao. A large part of the code comes from that project. See [Credits](#credits).

## Features

- **Search 24,000+ games** from Discord's own detectable-games list and keep the ones you care about in a sidebar.
- **Launch a stand-in for any game.** A few-KB dummy window is placed where Discord looks for the game, so there is nothing to download or install.
- **Steam library launch** for newer quests that pair a running game with a Steam install. The temporary Steam entry is tracked and removed again when you stop. See [How it works](docs/how-it-works.md).
- **Run several games at once.** Press **Select** above the game list, tick the games and press **Run selected**. The status card shows everything that is running (green) or idle (orange), with one Stop button. It stays in sync if you close a dummy window yourself.
- **Auto-stop.** Each game stops by itself after a set time (20 minutes by default, changeable per game) so a dummy is never left running.
- **Favorites, filter and "last used"** keep a long list easy to use. Shortcuts: `Ctrl+F` filters your games, `Ctrl+Enter` runs the selected games, `Esc` clears the filter or leaves Select.
- **Custom activity.** Show your own Rich Presence status, such as "Playing ..." or "Watching ...", with your own text and a timer, and save presets. It does not complete quests.
- **One window.** Starting the app a second time just brings the running one forward.
- **Warns you when Discord isn't running**, and tells you when a newer release is available.
- **Runs in the tray.** Optionally start with Windows, start minimized and hide to the tray when you close the window.
- **Steam entries panel** in Settings lists everything the app added to your Steam library, with Remove and Remove all. Entries are also removed when the app quits, unless you turn on "Keep Steam entries after a game stops".
- **Always up to date.** The game list is loaded from Discord first, then a daily GitHub Pages mirror, then a copy bundled with the app.
- **Settings page** for the theme (light, dark or follow Windows), startup and tray, auto-stop, cleanup, the game list and updates. A short **How to use** guide is in Settings too. A Diagnostics page with a Rich Presence test and the app log.
- **Report a bug or request a feature from the app.** The Feedback section in Settings opens a pre-filled issue on GitHub in your browser.

## Installation

Download the latest build from the [Releases](https://github.com/cmxau/discord-quest-completer/releases) page:

- **Installer** (`.exe` or `.msi`): installs like any other app.
- **Portable zip**: extract it somewhere you can write to. The app creates its dummy game files next to itself, so avoid places like `C:\Program Files\` or the root of `C:\`.

Requirements: Windows 11 (Windows 10 should work) and Microsoft WebView2, which ships with Windows 11. On older systems install it from [Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2).

> [!TIP]
> Windows SmartScreen and some antivirus programs may flag the app because it is unsigned and starts small helper executables. You can inspect and build the source yourself, see [Development](#development).

## Usage

1. Open the app and search for the game your quest asks for, then click **Add**.
2. Select it in the sidebar. If the game lists executables, press **Play** next to one.
3. For newer games that Discord pairs with Steam, use the **Steam library** card instead. Check the install folder and executable name, then press **Launch in Steam library**.
4. Check Discord: it should show the game as running. Keep the stand-in running while the quest completes.
5. Press **Stop** in the sidebar when you are done. Steam entries are cleaned up automatically.

## How it works

Discord matches running processes against the executable paths in its detectable-games list. This app
runs a tiny window from a matching path, so no game files are needed. Newer games are paired with a
store installation instead, which the Steam library feature covers. The details, limits and safeguards
are in [docs/how-it-works.md](docs/how-it-works.md).

## Uninstall

Remove the app from Windows Settings (installer) or delete its folder (portable). Optionally delete
the leftover data in `%APPDATA%\io.github.cmxau.discordquestcompleter` and
`%LOCALAPPDATA%\io.github.cmxau.discordquestcompleter`. The dummy game files are in the `games/` folder
next to the app and can be deleted at any time.

## Development

Requirements: [Rust](https://www.rust-lang.org/tools/install) with the
[Tauri prerequisites](https://tauri.app/start/prerequisites/), Node.js 20+ and
[pnpm](https://pnpm.io/). Windows is required to build the runner.

```bash
pnpm install
pnpm sync:runner     # builds the dummy game runner and copies it into src-tauri/resources/
pnpm tauri dev       # start the app with hot reload
```

Other commands:

```bash
pnpm build                                              # type-check and build the frontend
cargo test --lib --manifest-path src-tauri/Cargo.toml   # Rust unit tests
pnpm tauri build                                        # production build and installers
```

### Project structure

```
.
├── src/             Vue 3 + TypeScript frontend (components, composables, pages, utils)
├── src-tauri/       Tauri backend in Rust (process control, Steam library, Discord RPC)
├── runner/          The dummy game window (a tiny Win32 executable)
├── scripts/         Build helpers: bundle the runner, regenerate the app icon
├── docs/            Documentation
└── .github/         CI, release and Pages workflows, issue templates
```

Games can be added locally by editing `src/assets/custom-games.json`, which is merged over Discord's list.

## Contributing

### Bugs and feature requests

The easiest way is the **Feedback** section in the app's **Settings** (gear icon in the header):

- **Report a bug** opens the bug form with your app version, Windows version, the selected game and the recent app log already filled in.
- **Request a feature** opens the feature form with the app version filled in.

Nothing is sent until you review the page on GitHub and press **Submit**. Personal folder names (your Windows user name) are removed from the log before the link is made. You can also open [a new issue](https://github.com/cmxau/discord-quest-completer/issues/new/choose) yourself and pick the same forms. They live in `.github/ISSUE_TEMPLATE/`.

### Pull requests

Pull requests are welcome. Please run `pnpm build` and the Rust tests before opening one.

## Credits

This project would not exist without the original work:

- **[Mark Terence Tiglao](https://github.com/markterence)** created
  [discord-quest-completer](https://github.com/markterence/discord-quest-completer): the Tauri app, the
  dummy-runner approach, the game-list handling and the Rich Presence integration. This project is built
  on that code under its MIT license, and the original copyright notice is preserved
  in [LICENSE](LICENSE).
- Everyone who contributed to the original project, reported issues there or shared research on how
  Discord's quest detection changed.
- The r/DiscordQuests community, for documenting the Steam manifest method that the Steam library
  feature automates
  ([discussion](https://www.reddit.com/r/DiscordQuests/comments/1rhvwn9/method_for_completing_the_quests_for_marathon_and/)).

## Disclaimer

This tool is intended for educational purposes and personal use. Please respect Discord's terms of
service and the rights of game publishers and advertisers when using it. Automating quest completion
may violate Discord's terms, and your account could be restricted. The authors and maintainers are not
liable for any damages, account suspensions or other consequences of using this software. Use it at
your own risk.

This project is not affiliated with, endorsed by, or connected to Discord Inc. Discord is a registered
trademark of Discord Inc., referenced here for descriptive purposes only.

## License

[MIT](LICENSE) &copy; 2026 cmxau. Original work &copy; 2025 Mark Terence Tiglao.
