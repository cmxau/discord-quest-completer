# How it works

This document explains how Discord Quest Completer makes Discord think a game is running, what changed
in how Discord detects games, and the safeguards around the Steam library feature.

## Game detection in short

Discord keeps a public list of "detectable" applications
(`https://discord.com/api/applications/detectable`, roughly 24,000 entries). Each entry has an
application ID, a name and, for many games, a list of executables such as `win64/cs2.exe` or
`ea sports fc 26/fc26.exe`. The Discord client watches running processes and matches their paths
against those entries.

### The executable method

For entries that list executables the app:

1. creates `games/<app id>/<relative path>` next to the app, for example
   `games/1158877933042143272/win64/cs2.exe`;
2. copies the dummy runner (`data/quest-runner.exe`) to that path;
3. starts it with `--title "<game name>"`.

The runner is a tiny window (a few KB, no C runtime) so there is nothing to download or install. To
Discord it is a process with the right path, so the game shows as running and quest time counts.

## What changed: games without executables

Most newer entries have an empty `executables` list. Discord instead pairs the running process with an
installation recorded by a store, listed in the entry's `third_party_skus` (Steam, Xbox, Epic, ...). A
dummy exe in an arbitrary folder is no longer enough for these games.

For **Steam** games, a dummy exe placed in the game's Steam install folder, next to a Steam manifest,
is recognised. The app automates this in the **Steam library** card.

### The Steam library card

When you launch from the card the app:

1. looks up the install folder and launch exe for the Steam app ID (from the community
   [steamcmd.net](https://steamcmd.net) mirror of Steam's app info);
2. writes `steamapps/appmanifest_<id>.acf`;
3. copies the runner to `steamapps/common/<installdir>/<exe>` and starts it from there.

Some games (many EA titles, for example) launch through a URL, so the real exe name is not available.
The card pre-fills a guess based on sibling games (`EA SPORTS FC 27` becomes `fc27.exe`), marks it as
unverified, and lets you edit it.

### Safeguards

This touches your real Steam library, so the design is conservative:

- **Never touches a real install.** It refuses if Steam already has a manifest that looks like a real
  install (a build ID, a size, installed depots or a queued download) or if the install folder
  already contains files.
- **Everything is tracked.** Each file and folder it creates is recorded in
  `%APPDATA%\io.github.cmxau.discordquestcompleter\steam_fakes.json`, written before the files are
  created so a crash cannot leave untracked files behind.
- **Everything is removed again.** Pressing Stop (or the game exiting by itself) deletes the
  manifest, the exe and any folders it created. Folders that existed beforehand are kept. Anything
  left behind by a crash is removed the next time the app starts.
- **Paths are validated.** The install folder and exe name cannot contain separators, `..` or reserved
  characters, and cleanup refuses to delete anything outside `steamapps/common`.
- **No background downloads.** The manifest sets `AutoUpdateBehavior` to "only update when launched"
  so Steam does not start downloading the game.

### Known limits

- Steam may rewrite the manifest while it is open, and may show the game as installed while the dummy
  runs. Do not launch the game from Steam.
- The exe name is a guess for games Steam launches through a URL.
- Xbox and Epic exclusives have no equivalent here.
- Discord changes detection over time. Nothing here is guaranteed to keep working.

## Fallback executables

For games with no executables and no Steam link, the app generates a Windows executable name from the
title (`Shift at Midnight` becomes `ShiftAtMidnight.exe`) so there is still something to launch. These
rows are marked **Auto-generated**, and Discord will usually not detect them. They are a last resort.

## Rich Presence mode (experimental)

The **Test RPC** button connects to the local Discord client and sends a Rich Presence update using
the selected game's application ID, as if you were playing it. This is Rich Presence only. Quests do
not count it as the game running, and using another application's ID may go against Discord's terms.
Use it at your own risk.

## Game list sources

The list is loaded in this order, falling back at each step:

1. Discord's API directly (most up to date);
2. the GitHub Pages mirror, refreshed daily by `.github/workflows/pages.yml`;
3. `src/assets/gamelist.json`, bundled with the app.

`src/assets/custom-games.json` is merged on top of whichever list is used. Entries there replace
fetched entries with the same `id`, which is useful for fixing or adding a game locally. It must
follow the same shape as Discord's entries (`id`, `name`, `aliases`, `executables`).

## Credits and references

- The executable method, the dummy runner idea and the original app are from
  [markterence/discord-quest-completer](https://github.com/markterence/discord-quest-completer).
- The Steam manifest approach follows the manual steps described in that project's notes
  and in the r/DiscordQuests community
  ([discussion](https://www.reddit.com/r/DiscordQuests/comments/1rhvwn9/method_for_completing_the_quests_for_marathon_and/)).
  The research into how Discord's detection changed was shared by contributors to the original
  project's issue tracker.
