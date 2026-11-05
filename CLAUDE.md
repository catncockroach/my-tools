# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

A repo of personal Chrome extensions (Manifest V3, plain JavaScript, no build step, no dependencies, no tests). Each top-level folder containing a `manifest.json` is one extension. The user communicates in Vietnamese; UI strings, comments and READMEs are in Vietnamese.

Development happens in WSL, but Chrome runs on Windows, so extensions must be copied to the Windows drive before Chrome can load them.

## Commands

```bash
./sync-to-windows.sh                 # copy all extensions to C:\Users\haihv\Downloads\chrome-extension\
./sync-to-windows.sh youtube-tools   # copy one extension
WIN_EXT_DIR=/mnt/c/some/path ./sync-to-windows.sh   # override destination

# Syntax checks (the only "lint" available)
for f in youtube-tools/src/*.js youtube-tools/src/features/*.js youtube-tools/popup/*.js; do node --check "$f"; done
python3 -m json.tool youtube-tools/manifest.json >/dev/null
```

Every change to an extension must bump `"version"` in its `manifest.json`, in the same commit, so the user can confirm in `chrome://extensions` that the reload picked up the update. Bump the patch for fixes/tweaks (1.2.3 → 1.2.4), the minor for new features (1.2.4 → 1.3.0). Mention the new version number in the final reply.

After changing code: bump the version, run the sync script, then the user clicks Reload on the extension in `chrome://extensions` and refreshes the page. Changing the Windows folder path makes Chrome treat it as a new extension (new ID, settings reset), so keep the destination path stable.

When adding a new extension, add a row to the table in `README.md`.

## youtube-tools architecture

A feature-module system so new YouTube features can be added without touching existing ones.

- `src/features.js` defines the global `YTT_FEATURES` (id, name, description, defaultEnabled). It is loaded **both** by the content script and by `popup/popup.html`, so the popup renders its toggle list from it automatically.
- `src/core.js` defines the global `YTT`: the module registry (`YTT.register(id, {start, stop})`) and shared helpers (`isAdShowing`, `findSkipButton`, `skipAd`, `getVideo`, `isTyping`, `toast`).
- `src/features/*.js`: each file calls `YTT.register(<id>, { start(), stop() })`. The id must match an entry in `YTT_FEATURES`. Features must be fully reversible via `stop()`, because they are toggled live.
- `src/main.js` reads `chrome.storage.sync` key `features` (an object `{id: bool}`, with defaults from `YTT_FEATURES`), starts/stops modules, and reacts to `storage.onChanged` so popup toggles apply without reloading the page.

Content-script load order in `manifest.json` matters: `features.js` → `core.js` → `features/*.js` → `main.js`. A new feature needs three edits: an entry in `features.js`, a new file in `src/features/`, and that file listed in `manifest.json` before `src/main.js`.

Ad-skip details: an ad is detected via the `ad-showing` class on `#movie_player`. `skipAd` clicks the skip button, then after 300 ms, if the ad is still showing with a skip button, it seeks the ad video to its end, because YouTube may ignore synthetic (untrusted) clicks. Skip-button selectors live in `SKIP_SELECTORS` in `core.js`, and those selectors are the first thing to update when YouTube changes its markup.

## Commits

Whenever you are about to run `git commit` in this repo (the user says commit, commit đi, tạo commit, lưu thay đổi, or a task ends with committing), load the `commit` skill (`.claude/skills/commit/SKILL.md`) yourself and follow it. Do not wait for the user to type `/commit`. It defines the house style: an all-lowercase `brief title: detail` subject, `- ` bullet lines, and the Co-Authored-By trailer.

Commit proactively: when a change is complete (code edited, syntax-checked, synced to Windows), commit it without waiting to be asked. Never push unless asked.
