# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`my-tools`: a repo of personal tools, one top-level folder per tool. Two kinds so far:

- **Chrome extensions** (Manifest V3, plain JavaScript, no build step, no dependencies, no tests). Each top-level folder containing a `manifest.json` is one extension.
- **Windows apps in Rust**: `screen-hud/` (see its section below).

The user communicates in Vietnamese; UI strings, comments and READMEs are in Vietnamese.

Development happens in WSL, but Chrome and the Windows apps run on Windows, so code must be copied to the Windows drive before it can be loaded or built.

When adding a new tool, add a row to the table in `README.md`.

## Chrome extensions

### Commands

```bash
./sync-to-windows.sh                 # copy all extensions to C:\Users\haihv\Downloads\chrome-extension\
./sync-to-windows.sh youtube-tools   # copy one extension
WIN_EXT_DIR=/mnt/c/some/path ./sync-to-windows.sh   # override destination

# Syntax checks (the only "lint" available)
for f in youtube-tools/src/*.js youtube-tools/src/features/*.js youtube-tools/src/page/*.js youtube-tools/popup/*.js; do node --check "$f"; done
python3 -m json.tool youtube-tools/manifest.json >/dev/null
```

Every change to an extension must bump `"version"` in its `manifest.json`, in the same commit, so the user can confirm in `chrome://extensions` that the reload picked up the update. Bump the patch for fixes/tweaks (1.2.3 → 1.2.4), the minor for new features (1.2.4 → 1.3.0). Mention the new version number in the final reply.

After changing code: bump the version, run the sync script, then the user reloads the extension (for youtube-tools: click the version chip in the popup, which calls `chrome.runtime.reload()` and shows `↻ vX` when the synced manifest on disk is newer; otherwise Reload in `chrome://extensions`) and refreshes the page. Changing the Windows folder path makes Chrome treat it as a new extension (new ID, settings reset), so keep the destination path stable.

### youtube-tools architecture

A feature-module system so new YouTube features can be added without touching existing ones.

- `src/features.js` defines the global `YTT_FEATURES` (id, name, description, defaultEnabled). It is loaded **both** by the content script and by `popup/popup.html`, so the popup renders its toggle list from it automatically.
- `src/core.js` defines the global `YTT`: the module registry (`YTT.register(id, {start, stop})`) and shared helpers (`isAdShowing`, `findSkipButton`, `skipAd`, `getVideo`, `isTyping`, `toast`).
- `src/features/*.js`: each file calls `YTT.register(<id>, { start(), stop() })`. The id must match an entry in `YTT_FEATURES`. Features must be fully reversible via `stop()`, because they are toggled live.
- `src/main.js` reads `chrome.storage.sync` key `features` (an object `{id: bool}`, with defaults from `YTT_FEATURES`), starts/stops modules, and reacts to `storage.onChanged` so popup toggles apply without reloading the page.

Ad blocking at the source: `src/page/block-ads.js` is a separate content script with `"world": "MAIN"` and `run_at: document_start`, so it runs before YouTube's scripts. It removes `adPlacements`/`adSlots`/`playerAds` from player data via a setter on `window.ytInitialPlayerResponse` and Proxies on `Response.prototype.json` and `JSON.parse`. MAIN-world code cannot use `chrome.*`, so the `blockAds` feature module (isolated world) only writes a flag to youtube.com `localStorage` (`ytt-block-ads`, `"0"` = off), which the page script reads on every call. `main.js` calls `stop()` on initially-disabled features so such flags get written, so every `stop()` must be safe to call before `start()`.

Content-script load order in `manifest.json` matters: `features.js` → `core.js` → `features/*.js` → `main.js`. A new feature needs three edits: an entry in `features.js`, a new file in `src/features/`, and that file listed in `manifest.json` before `src/main.js`.

Ad-skip details: an ad is detected via the `ad-showing` class on `#movie_player`. `skipAd` clicks the skip button, then after 300 ms, if the ad is still showing with a skip button, it seeks the ad video to its end, because YouTube may ignore synthetic (untrusted) clicks. Skip-button selectors live in `SKIP_SELECTORS` in `core.js`, and those selectors are the first thing to update when YouTube changes its markup.

## screen-hud (Windows, Rust)

A tiny always-running background process that shows a faded, click-through overlay in the bottom-right (or, per a tray menu setting, bottom-left) corner of the screen (above the taskbar). Today it shows battery % of connected Bluetooth devices. **Minimal RAM/CPU of the background process is the primary requirement**: no webview or heavy runtime in it, no render loop (redraw only when content changes), event-driven where possible, and measure with `./dev.sh mem` after changes. A future settings window (Tauri) is meant to run as a separate, on-demand process so the background process never loads WebView2.

```bash
screen-hud/dev.sh build        # rsync source to C:\Users\haihv\dev\screen-hud, cargo.exe build --release there
screen-hud/dev.sh run [--all]  # kill running instance, build, start it with --portable
screen-hud/dev.sh list         # build, print devices (name, %, connected) to stdout
screen-hud/dev.sh mem          # RAM/handles/CPU time of the running instance
```

The Rust toolchain is installed on Windows only, not in WSL; building from the `\\wsl$` path is slow, hence the rsync. WSL interop sometimes fails at process start with `UtilAcceptVsock: accept4 failed 110`; `dev.sh` retries those calls.

Local Windows toolchain is `stable-x86_64-pc-windows-gnu` (minimal profile, self-contained linker). Visual Studio Build Tools were deliberately not installed (too heavy for the user's laptop); do not install them. CI (`.github/workflows/screen-hud.yml`, `windows-latest`, MSVC) builds on every push/PR touching `screen-hud/` as a compile check (no artifacts); pushing a tag `screen-hud-vX.Y.Z` (must equal the `Cargo.toml` version) also creates a GitHub Release with the exe. `Cargo.lock` is committed (`dev.sh build` copies it back from the Windows build dir) because CI builds with `--locked`.

Every change must bump `version` in `screen-hud/Cargo.toml` in the same commit (same patch/minor rules as extensions).

Architecture (`screen-hud/src/`), plain Win32 via `windows-sys`:

- `battery.rs`: battery % is the PnP property `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` (what Settings > Bluetooth shows; HFP headsets and BLE devices). It is the last value Windows received and persists after disconnect, so it is joined by ContainerId with the Device Container properties `System.Devices.Connected` and `System.ItemNameDisplay`. Do not use Association Endpoint (AEP) queries: a Bluetooth AEP query (DevGetObjects or WinRT FindAllAsync, even filtered on IsPaired) runs a ~30 s radio scan per protocol before returning. Read with DevQuery (`DevGetObjects`), not WinRT, to avoid loading WinRT. `watch()` registers `DevCreateObjectQuery` callbacks that post a message to the UI thread; plus a 60 s poll for battery changes.
- `overlay.rs`: layered window (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`). Text is drawn with GDI as white-on-black grayscale AA, the luminance becomes the alpha mask, and the backdrop/shadow/colour are composed by hand into premultiplied BGRA for `UpdateLayeredWindow`. The DIB is freed right after.
- `install.rs`: the exe installs itself, no installer tooling. Run from anywhere other than `%LOCALAPPDATA%\Programs\screen-hud` (and without `--portable`), it closes a running instance (finds the `ScreenHudCtl` window, posts `WM_CLOSE`, waits), copies itself there, writes the HKCU Run and Uninstall keys, launches the installed copy and shows a message box. That same path is the update flow (download the new exe, run it). `--uninstall` (Settings > Apps) removes the keys and `HKCU\Software\screen-hud`, then deletes the folder via a delayed `cmd` after the message box closes. Settings live in `HKCU\Software\screen-hud` (`ShowOverlay`, `OverlayLeft`). `dev.sh run` always passes `--portable`, so dev builds never install.
- `tray.rs`: tray icon and context menu. The icon is a duck (the app's logo) rasterized once at startup from a few ellipses (`DUCK`), 4x4 supersampled, with a dark outline so it reads on light and dark taskbars; no `.ico` resource.
- `main.rs`: a hidden control window (tray, timers, DevQuery messages) plus the overlay window; state in a `thread_local` `RefCell` (borrow briefly, never across calls that can re-enter the wndproc such as `TrackPopupMenu`). Hides itself when a fullscreen app runs (`SHQueryUserNotificationState`, 2 s timer). Single instance via the `Local\screen-hud` mutex.

## Commits

Whenever you are about to run `git commit` in this repo (the user says commit, commit đi, tạo commit, lưu thay đổi, or a task ends with committing), load the `commit` skill (`.claude/skills/commit/SKILL.md`) yourself and follow it. Do not wait for the user to type `/commit`. It defines the house style: an all-lowercase `brief title: detail` subject, `- ` bullet lines, and the Co-Authored-By trailer.

Commit proactively: when a change is complete (code edited, syntax-checked, synced to Windows), commit it without waiting to be asked. Never push unless asked.
