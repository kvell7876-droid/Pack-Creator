# Pleasure Horizons Pack Creator

An IDE for Pleasure Horizons packs. A project is one data pack (a folder): its data is in `data/<namespace>/...` and what the game sends to players (models, textures,
sounds, animations) is in `data/<namespace>/asset_sync/...`, as Strut's asset sync defines. There is no separate resource pack.

The plan and what is built so far: `AI Info/PACK_CREATOR_PLAN.md`. The first tool (Node and Electron) is archived in `tools/archive/custom_pack_creator_legacy`.

## Layout

| Folder | What is in it |
|---|---|
| `crates/pc_core` | The logic, with no window: projects, pack layout, the game reference, JSON Schemas for the editor, the checks. `cargo test -p pc_core` |
| `src-tauri` | The window (Tauri 2). It lends `pc_core` to the page as commands and holds which project is open. |
| `ui` | The page: Preact and TypeScript, with Monaco (the VS Code editor) for text. |
| `docs` | The guides shown by the Docs button (markdown, put into the program when the page is built). The reference pages are made from `reference.json`, not written. |

## The test pack

`cargo test -p pc_core build_test_pack -- --ignored --nocapture` makes `test_pack/` from the girls of the old resource packs in the game's `run/resourcepacks` folder (with their profiles from `run/config/pleasurehorizons/girl`): about twenty real girls with models, scenes, animations, cues and physics, to try the viewer, the scene simulator and the physics strip on. It is not kept in git because the girls are other people's work. The body type of each is guessed from how like the game's own skeletons her bones are. The checks it ends with are the tool's real findings on those packs.

## Run it

Needs Rust (MSVC toolchain) and Node. Once: `cd ui && npm install`.

- **Develop the page in a browser** (no window; a small in-memory stand-in answers for the Rust core): `cd ui && npm run dev`, open http://localhost:5173.
- **Develop the real app:** `ui/node_modules/.bin/tauri dev` from this folder.
- **Build the program:** `ui/node_modules/.bin/tauri build --no-bundle`. The result is `target/release/pack-creator.exe`, one file that needs only the WebView2 runtime that Windows 11 has.
- **Tests:** `cargo test -p pc_core` (includes a check that the game's own girls, scenes, traits and dialogue raise no errors).

## Looking at the real window

The page can be developed in a browser, but the real window is checked with `scripts/drive.mjs`: start the exe with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`,
then `node scripts/drive.mjs eval "<js>"` runs script in the page (the real commands are `window.__TAURI_INTERNALS__.invoke(name, args)`), `shot file.png` saves a screenshot, and
`key`, `click` and `text` send real input.

## The game reference

Everything the checks and the editor's suggestions know about the game (components and their fields, traits, goals, scene settings) is in
`crates/pc_core/reference/reference.json`. It is the file `/pleasure dev girlSchema` writes, plus two hand-written sections (`scene`, `trait_file`) until the game
exports them. When the game gains a setting, replace the file and the tool knows it. Known gap: the game's export lists the `tempt` goal without its `item` setting; the
reference has it added by hand.
