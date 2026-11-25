# Changelog

_[Español](CHANGELOG.es.md)_

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Shortcut `Alt+E` toggles between reading and editing (besides `F2`): one press enters editing and the next returns to reading.
- License: Synapse.rs is now free software under GPL-3.0-or-later. `synapse-server/` (only a proof of concept), the name and the logos are not included.
- Agent chat: the account quota left is shown next to the context (`● 58% wk`), with every limit in the tooltip. Claude attaches it to the conversation; for Antigravity it is asked with `agy`, at most every 3 minutes (configurable), and `/usage` includes it.
- Agent chat: the icon of each agent in the ACP registry is shown before its name; unknown agents get the generic robot.
- Agent chat: when a conversation is reopened, the context bar is shown without waiting for the first answer.
- Explorer: files and folders dragged from another application (Nautilus, Explorer, Finder…) are copied into the folder where they are dropped.

### Fixed

- Explorer (Linux): dragging files inside the explorer, or onto the agent chat, works again.
- Notes: switching between reading and editing in the middle of a long table keeps the visible row instead of jumping to the start of the table.
- Agent chat: choosing a file with `@` keeps its reference in the text (`@docs/note.md`) instead of deleting what was typed; it is still attached.
- Linux notifications: the octopus is also sent as the notification image, for daemons such as swaync.

### Security

- Dependencies updated. The Excel viewer moves to `umya-spreadsheet` 4 (beta), which drops a `zip` version with known vulnerabilities.

## [1.0.0-beta.9] - 2026-11-24

### Added

- Command palette (Ctrl+P): names follow the interface language, and commands can be found in both English and Spanish (`update`, `upgrade`, `cerrar pestaña`…). The closest match comes first.
- What's new and update notes in English or Spanish, following the interface. The changelog now has both versions.

### Fixed

- Windows: images opened as a file were not displayed when the vault was opened from recent vaults or the terminal, because its path carried the `\\?\` long-path prefix.

### Changed

- English is the base language: text missing in Spanish is shown in English, and Synapse starts in English when the system uses any other language.

## [1.0.0-beta.8] - 2026-11-24

### Security

- Excalidraw drawings in the plugin API: a tampered file could run code when displayed. `toSVG` escapes its values and the view sanitizes the SVG.
- Plugin API: `sanitizeHTMLToDom` now truly sanitizes HTML and `htmlToMarkdown` no longer leaves reconstructed tags behind.
- File access: the interface can only read and write inside the open vault and the files you choose (dialog or command line); before, any path. A loose file also shows the images from its folder and subfolders. A vault that is not in the recent list asks for confirmation. A wallpaper chosen in a previous version has to be chosen again, and images with an absolute path outside the vault are no longer shown.
- Saving a custom agent or entering a different Git path asks for confirmation with a system dialog that shows the program that will be run.
- Agent installer: special permissions (SUID/SGID/sticky) in a ZIP are discarded.
- Dependencies with known vulnerabilities and a compatible patch, updated (`url`, `cookie` and `lodash-es`).

### Fixed

- Git Log of all branches: the context menu briefly appeared in the wrong place and then jumped to its position, so a quick click could land on a different option.
- Explorer with many files and an interface zoom other than 100%: moving through the list or opening a file froze the app for several seconds, because the row height oscillated and the list was remounted hundreds of times.
- Folder indexes (`index.md`): the lock and the Git status stay next to the names and aligned in a column, instead of at the right edge; the description follows.
- Agent chat: with Claude the model selector did not appear, because it already announces it in `configOptions`. Synapse reads that format and changes the model with `session/set_config_option`.
- Unit tests (`bun test`): the plugin that compiles the `.svelte.ts` files strips the types with Bun's transpiler, because TypeScript 7 no longer ships `transpileModule`; the `test` script only looks at `./src` (it no longer picks up the `zennotes/` folder).
- Agent chat: text typed but not sent is no longer lost when the panel closes (for example, when unpinned, on clicking outside) or when the conversation is folded.
- Internal links with reserved characters (`%2C`, etc.) in reading view and OKF `index.md`: they are decoded with `decodeURIComponent` instead of `decodeURI` so that files with commas or other special characters in the name (such as `.excalidraw`) open correctly.
- Agent chat: fixed a bug where messages queued while the agent was working were not sent when the turn finished.
- Resizable dialogs (note Outline `Ctrl+F12` and Git dialogs): the resize indicator is shown in the bottom-right corner, the grab areas on edges and corners are enlarged and the pointer is captured while dragging so that resizing is reliable.
- Agent chat: collapsing a reasoning block now shows "Reasoning…" next to the icon instead of just the emoji, and expanding it removes the excess whitespace before the tool cards.
- Agent chat: the `file://` links in its replies did not open anything (the HTML sanitizer stripped their target). Now they open the vault file, and a `#L56-L65` anchor opens it in the editor with those lines selected. If the path does not fall inside the vault (another root or another copy of the repository), it is looked up by the end of the path in the index: one match opens, several let you choose.

### Added

- Agent chat: effort level selector next to the model, with the agents that offer it (Claude, agy). It is remembered per agent. With agy, the effort is no longer repeated in the model list.
- Markdown notes: in reading view, headings and subheadings fold with the arrow to their left (Alt+click, all of that level; `zM`/`zR`, all of them). Search, the outline and links to a heading unfold what is needed. "Fold / Unfold all headings of the note" commands, also in edit mode.
- When the agent finishes, besides the sound, Synapse sends a **system notification with the kawaii octopus** and the query that finished (`Query finished: "…"`) if the instance running it does not have focus (for example, if you are in another application). The agent's permission and question prompts now use the same icon.
- **`.sql`** files are a supported type: they appear in the explorer and in Ctrl+O, are indexed in content search as code and are edited with SQL syntax highlighting (also in the Git diff).
- Built-in **GitHub** theme (green headings on a near-black background in dark; white in light), also as "GitHub Islands".
- Built-in **Solarized** theme (cream in light, teal in dark, cyan headings), also as "Solarized Islands".
- Built-in **Willemstad** theme (warm black in dark, white and sage in light, headings in green stripes), also as "Willemstad Islands".
- Quick theme switching: `` Ctrl+` `` or the "Pick theme…" command opens the theme list with search, without going into Settings. The shortcut can be changed (`pick_theme`).
- Global search and Quick Open: when opening a result, the editor's finder (Markdown in reading and edit modes, YAML/JSON and text) is loaded with the searched term and positioned on the first match.
- Agent chat (ACP): `/btw <question>` asks a side question without interrupting the current turn. The answer appears in a floating panel above the text box, does not enter the conversation or the history and is closed with Esc; "Convert to conversation" continues it separately. It rejects any permissions the agent requests.
- Agent chat (ACP): `/fork [message]` opens another conversation with the context of the current one, also while a turn is in progress. `/btw` and `/fork` use `session/fork` if the agent announces it (Claude) and, if not, forward the conversation to it as text (agy); the command hint says which of the two applies.
- Tab bar: "Close all tabs" option in each tab's context menu.
- Agent chat (ACP): sound when a turn finishes (`Burbuja FM` by default), configurable in Settings → AI Agents with the options `Burbuja FM`, `Sonar orgánico` or `Sin sonido` and a "Test" button.
- Agent chat (ACP): full support for `elicitation/create` in `form` mode (interactive form generated from JSON Schema with buttons/selects for enums, booleans, numbers, text and required-field validation) and `url` mode (shows the domain, URL and an "Open link" button), announcing `clientCapabilities.elicitation` in `initialize` and replying with `accept`, `decline` or `cancel`.
- Text diff viewer (`@codemirror/merge`): navigation between change blocks (↑/↓ with a counter), built-in search (`Ctrl+F` and a button in the bar), direct editing of the file on disk in working-tree diffs synchronized with the Reading/Edit button (`Ctrl+E`) with buttons to revert individual blocks (`→` in split view, "Revert" / "Keep" in unified), autosave and full support for the CodeMirror editor (Vim mode, folding, comments, rename occurrences, line shortcuts, autocompletion and `Ctrl+Z` history), plus syntax highlighting and diff colors adapted to light and dark themes.
- Repository Git Log: columns can be reordered by dragging their headers (order saved) and "Hash" becomes the first column by default.
- Welcome screen: pinned vaults can be reordered with drag and drop (order saved in `workspaces.json`) and pinned ones with a Git repository keep the Git folder icon with the pinned color.
- Markdown outline and YAML/JSON keys: the outline popover reuses `@shared/ui/dialog-resize` so it can be resized by dragging its edges or corners and remembers its size.
- Folder index (`index.md`): "+ New…" (dropdown: Markdown note, Mermaid, D2, PlantUML, Excalidraw) next to "Files"/"Concepts" and "+ Folder" next to "Folders", which create inside that folder; if the section is missing, the buttons go at the top.
- Explorer: clicking a folder opens its `index.md` (clicking the folder icon or any area to its left —arrow or indentation— expands/collapses it; without an `index.md`, it expands as before). Settings → "Click on an explorer folder" lets you go back to "Expand it". In that mode the `index.md` is not listed inside its folder (the root's is); it remains on disk and in the MCP. "Locate active file" (Ctrl+Alt+1) on an `index.md` selects its folder without expanding it.
- Editor: hovering over a Git gutter mark shows a tooltip with what was in Git and what is there now (−/+), with the characters that changed highlighted (even if just one); for a deletion, the lines that were removed. "Revert" buttons (returns the block to how it is in Git; Ctrl+Z undoes it) and "Copy original". Own colors in light and dark (like the GitHub diff), readable with any theme. The tooltip appears next to the line under the mouse and stays open when you move onto it.
- Agent telemetry: global per-iteration usage log in `~/.config/synapsers/agent-usage.jsonl` (model, session, duration, tokens/cost and references to each vault's JSON files) and a detailed per-conversation sidecar in `<vault>/.synapse/agent-sessions/telemetry/<id>.json` with iteration times, tool usage and request/response correlation of calls to the Synapse MCP.
- Markdown tables: cells with `[ ]`, `[x]` or `[X]` are rendered as interactive checkboxes in the live editor, the visual cell editor and reading view.
- Agent chat: your messages are shown in Markdown (bold, lists, code, links…) with mentions highlighted, and have a "Copy" button that copies the text exactly as you wrote it so you can reuse the prompt.
- The tab of an `index.md` shows the name of its folder (the vault's for the root one) with a folder icon.
- Images and attachments (any link that is not a `.md` note) are also looked up in the `attachments/` folder next to the note: for `a/b/c/NOTE.md`, first `a/b/c/image.png`, then `a/b/c/attachments/image.png`, and then the usual rule (vault root, unique by name or choose).
- Agent panel with pin, like the explorer: pinned, it takes up its space on the right; unpinned, it floats over the editor without taking space from it and hides when you click outside or press Esc.
- OKF index (Open Knowledge Format v0.3) in every folder: Synapse maintains an `index.md` with its subfolders, concepts (notes with `type`, with their `description`) and other files, so an agent can walk the vault from disk. It is regenerated when the folder changes and only written if it changes (meant to be committed); at the root it carries `okf_version: "0.3"` and keeps `title`, `description`, `license` and `x-…`. A custom `index.md` is renamed to `index.deprecado.md`. In Synapse it opens read-only (without the read/write button) and does not appear in search; a link to a folder (`folder/`) opens its `index.md`. `SYNAPSE_OKF_INDEX=0` disables it.
- MCP: `read_note` also returns the root `index.md` and the one of the note's folder.
- Text diff: a line indicates what each side is (◀ before · after ▶, or − before / + after).
- Single-rule links in notes (wikilinks and Markdown links), relations panel and agent chat: if the path exists as written (relative to the note or to the root) it is used; if not, the only file with that name; if there are several, a list to choose from (↑/↓; Enter or click opens; Ctrl+Enter, Ctrl+click or "Pin" opens and fixes the link in the note with the full path). An ambiguous embedded image shows a notice with "Choose which".
- Agent chat: images open in a tab ("Agent · …", with the agent's icon) with the image viewer (zoom, pan, fit) and the "Save to vault" button.
- MCP: `get_backlinks_and_relations` reports `resolved_path` on each outgoing link and, if it is ambiguous, `ambiguous` with the candidates.
- Agent chat: while the agent is working, Enter queues the message (with its attachments and mentions) and it is sent automatically when the turn finishes, in order. Each queued message has ✕ to remove it, ↑ with an empty box pulls out the last one to edit it, and "Stop" cuts the turn and returns the queued messages to the box without sending them. It works with any ACP agent (Synapse manages the queue).
- Agent chat: replies show task checkboxes (the HTML sanitizer used to strip them); under each reply, "Markdown" copies the text as is and "Formatted" copies it for an email or Teams, with tasks as ☐ / ✅; copying a selection with tasks does the same.
- Agent chat: zoom for messages and the text box with Ctrl+wheel, Ctrl++, Ctrl+− and Ctrl+0 (remembered).
- Agent chat: the text box grows as you type, up to 40% of the window height; after that it has its own scroll.

### Changed

- Folder index (`index.md` OKF): the lists of folders, concepts and files are rendered in reading view with an appearance similar to the explorer (folder and file-type icons, subtle metadata, Git status and lock, and a fully clickable row) without modifying the physical OKF file on disk.
- Text diff viewer: migrated from `@git-diff-view/svelte` to `@codemirror/merge` to improve performance on large files.
- Context menus of the vault explorer and breadcrumb: all Git options now appear grouped together at the top.
- Resizable dialogs unified on `@shared/ui/dialog-resize` in `QuickOpen`, `FullTextSearch`, `CommandPalette`, `SettingsPanel`, `ShortcutsPanel` and `CommunityPluginsModal` (keeps saved sizes, a grip in the bottom-right corner and dragging from the header in community plugins).
- Whether the explorer and the agent panel are pinned is remembered per vault (the first time, each vault takes the last global value).
- Visual diff "2 Panels": left = before (Git, previous version, first file) and right = after (disk, new version), like the text diff; the editable disk panel moves to the right.
- Saving a Markdown note no longer adds `title` to the frontmatter (nor creates a frontmatter just for that): the default title is the file name. On rename, the `title` only changes if it already existed and matched the previous name; a custom title is respected. The MCP's `write_note` only writes `title` if it is passed.
- The text diff opens collapsed, showing only the changes ("Expand all" in the bar to see it in full). `diff.max_expand_lines` in config.jsonc is now 0; with a larger number, files up to that many lines open expanded as before.
- A link with several files of the same name no longer silently goes to the "closest" one: it asks. Those ambiguous links do not count in backlinks, graph or relations until they are pinned.
- OpenAPI viewer: Scalar no longer uses the window URL (`#…`); with several OpenAPI tabs each one works on its own (before, a new tab would jump to the other's section).

## [1.0.0-beta.7] - 2026-11-20

### Added

- Zoom in the Word and spreadsheet viewers that enlarges everything, as in a PDF (not just the font): in Word the whole page, with "Fit to width"; in spreadsheets, column widths, row heights, font and headers, while still painting only what is visible. From 50% to 300% with the bar control, Ctrl+wheel or the size shortcuts (Ctrl++, Ctrl+−, Ctrl+0), which inside these viewers perform this zoom. One zoom is remembered for all Word files and another for all spreadsheets.
- When copying a selection with tasks from Markdown reading mode, the checkboxes are copied as ☐ (pending) and ✅ (done), with the list and bold, to paste into an email or Teams web (which strip HTML checkboxes); also in plain text (`- ☐ task`). Without tasks it is copied as usual.
- Configurable shortcut "Toggle task" (`toggle_task`, Ctrl+Enter by default) in the Markdown editor, as in Obsidian: toggles `- [ ]` ↔ `- [x]` on the cursor line (or on all lines of the selection), adds the checkbox to a list item that lacks one and turns a normal line into a task. It also works with Vim, in normal and insert mode.
- Word document viewer (`.docx`, `.docm`, `.dotx`) inside Synapse, read-only, with docx-preview (loaded when the first one is opened): pages, fonts, colors, tables, images, headers and footers. The old `.doc` still opens with the system application.
- "Open in application" button in the Word and spreadsheet viewers, to edit with the system application.
- Spreadsheet viewer inside Synapse (before, they opened with the system application), read-only: `.xlsx`, `.xlsm`, `.xltx`, `.xltm` with styles (bold, colors, fills, borders, alignment), merged cells and widths/heights; `.xls`, `.xlsb`, `.ods` with values and formulas only. Large workbooks open without freezing: each sheet is read when opened, cells arrive in row chunks as you scroll, only the rows and columns in view are painted, and the parsed workbook is cached while in use; an `.xlsx` over 10 MB is read without styles to open twice as fast. Formula bar with the selected cell, "Formulas" button (Ctrl+`) to see them in the grid, a notice on formulas without a saved result, sheet tabs at the bottom (hidden ones only when requested; Alt+PageUp/PageDown switches sheets), arrow-key navigation and Ctrl+C to copy the cell.
- Update a branch that is behind its remote without leaving Synapse (fast-forward, without creating commits): the branch badge shows ↓N / ↑N, the branches dialog has "Update from origin/…", and the Git Log menu offers "Fast-forward "branch" to here" on later commits. If the branches have diverged nothing is touched and the reason is explained.
- "Zoom: entire interface" command in the palette (Ctrl+P): a small selector from 70% to 200% operated with ↑/↓, Enter and Esc, with a preview as you move; it is saved in settings. The content text zoom remains in Settings and the OpenAPI viewer now follows it too (font only, like Markdown and the editor).
- The OpenAPI viewer follows `$ref`s to other vault files (`./paths/x.yaml`, `../schemas.yaml#/Pet`), also in Git revisions; it warns about missing references and refreshes when a referenced file is saved.

### Changed

- The OpenAPI viewer no longer shows Scalar's "Ask AI": the chat sent the specification to external servers.
- **App icon**: the icons in `src-tauri/icons/` are regenerated from `src/shared/assets/synapsers.svg`.
- **Windows CI**: the cross-compilation dependencies step installs only `clang lld llvm nasm` (the target is Windows; the Linux GTK/WebKit libraries were unnecessary) and the APT cache, which had no effect, is removed. It saves about 7 minutes per release.
- **xwin cache**: the cached path was `~/.cache/xwin`, which `cargo-xwin` does not use; it is now `~/.cache/cargo-xwin`, so the CRT and the MSVC SDK (~8 min download) are reused between runs.

### Fixed

- The `Buffer` that Synapse offers to Obsidian plugins took any `Uint8Array` for a `Buffer`, which made JSZip misread the file names inside a zip (for example when opening a `.docx`).
- The Git Log context menu appears next to the pointer (before, it appeared offset) and does not go off the window.
- "Checkout this commit" in the Git Log failed on confirming because it read the commit from the already closed menu.

## [1.0.0-beta.6] - 2026-11-20

### Added

- **In-app updates**: at startup (can be disabled in *Settings → Updates*) or with "Synapse: Check for updates" in the palette, Synapse checks the latest release and, if there is a new version, notifies at the bottom right with "See what's new", "Later" and "Update". The download is verified with the release signature and the executable is replaced (on Linux and Windows the release publishes the bare binary, which the Tauri installer does not know how to install); afterwards, "Restart now". `latest.json` now carries the version's release notes.
- **What's new**: "Synapse: What's new (changelog)" opens the change history in an in-memory tab (not a vault file). The first time a new version is opened, what's new since the version you were using opens automatically.
- **Configurable Git executable**: if the vault is a repository and Git is not found, a dialog lets you choose the executable (or "Don't ask again"); the path is saved in `config.jsonc` (`git.path`) and changed in *Settings → Git*, which shows the version in use and lets you go back to the system one. All operations that use the real Git respect it, and on Windows they no longer open a console window per command.

### Fixed

- **Vault with Git not detected (Windows)**: if gix opens the repository but fails to read the file status, the vault is still shown as a repository (branch and Git actions, without change marks) and the reason is left in the app log. The paths of changed files use `/` on Windows too (they came out with `\` and the explorer did not mark them).
- **Updater public key**: it was in `tauri.conf.json` as the inner minisign line (`RWQ…`) instead of Tauri's format (the key file in base64), so no download could have been verified and installed. It is the same key: already published releases verify with it.

- **Agent e2e without network**: the agent list waited up to 30 s for the public ACP registry (with no network or a hung CDN), and the agent panel test failed. The e2e tests point `SYNAPSE_ACP_REGISTRY_URL` to a closed port and use only local and custom agents.

### Changed

- **e2e**: the WebDriver port is configured with `E2E_DRIVER_PORT` (default 4444; WebKitWebDriver uses the next one), so that several test processes can run at once.

## [1.0.0-beta.5] - 2026-11-20

### Added

- **Verifiable release**: each release attaches `SHA256SUMS.txt`, the CycloneDX SBOM of the Rust backend and the frontend, and a provenance attestation (Sigstore, `gh attestation verify`) for each package. The packages and the updater signatures do not change.
- **`Security` workflow**: `cargo audit`, `cargo deny` (advisories, GPL-compatible licenses and sources, with `src-tauri/deny.toml`) and `bun audit` on every push to `main`, on PRs and once a week. CodeQL (JavaScript/TypeScript and Rust; Rust does not block) and OpenSSF Scorecard in their own workflows.
- **Weekly Dependabot** for Cargo, bun and GitHub Actions, with minor and patch updates grouped into one PR per ecosystem.
- **Tree view in the commit dialog**: a button next to "Select all" switches the file list to a folder tree (single-folder chains are compacted, like `docs/guides/`). Each folder folds and has its checkbox, which checks or unchecks everything it contains; the chosen view is remembered.
- **"Git Log…" and "Branches…" in the explorer**: right-clicking the empty space in the explorer, the vault name or the branch badge opens the repository log or its branches. Before, if the vault was a single repository, the log could only be opened from the command palette.
- **`SECURITY.md`** (how to report and how to verify a binary) and a "Security and privacy" section in the README with the map of the app's network connections.

### Changed

- **Presentation README**: the README is now the product presentation (before `docs/about.md`, which remains as a link to it); the development documentation moves to `CONTRIBUTING.md`. It no longer says "100% local": Synapse is local-first and a new section, "What leaves your machine", details which features use the network (draw.io, PlantUML, AI agents, plugins, Git) and what travels in each. The landing page (`docs/index.html`) is corrected the same way.
- **Workflows with minimal permissions**: `contents: read` by default and `contents: write`, `id-token` and `attestations` only in the job that publishes the release. Actions are pinned by commit SHA and `dtolnay/rust-toolchain` and `taiki-e/install-action` now receive explicit `toolchain` and `tool`.
- `src-tauri/Cargo.toml`: `publish = false` (the crate is not published to crates.io; cargo-deny needs it so as not to require a license from it).

### Fixed

- **Frontend SBOM in the release**: it is generated with syft over the production dependencies. `cyclonedx-npm` did not work with bun (`Unexpected npmJsPath`) nor with npm (conflict with the `overrides`), and the "SBOM" job blocked publication.
- **Versions of an OpenAPI in Git history**: "View file" shows the API with Scalar, as in Reading, instead of the YAML/JSON as plain text; the Raw button still shows the text.
- **Remote branch checkout**: "Checkout" on a remote branch without a local branch creates the local one with tracking (`git checkout -b feature/x --track origin/feature/x`) instead of leaving HEAD detached. And a local branch with `/` (`feat/y`) can no longer jump to another local branch called `y`.
- **Raw view of a version**: it no longer draws a horizontal line under each line of text (it inherited the inline `code` style).

## [1.0.0-beta.4] - 2026-11-19

### Added

- **Search in BPMN diagrams**: from a `.bpmn` file the name of each step is indexed with its type (task, event, gateway…), its documentation and the annotations, without the drawing coordinates; new `kind:bpmn` filter. The index is rebuilt automatically when the vault is opened.
- **The graph and the laser follow the theme**: nodes, names and links of the graph (2D and 3D) and the laser trail take their colors from theme variables (`--graph-*`, `--laser`) and are repainted when the theme changes. In the dark theme the note names in the graph are now readable (before, dark gray on a dark background).
- **GitHub Copilot agent via ACP**: appears in the agent list next to Claude and Antigravity. It uses the already installed `copilot` CLI (`copilot --acp`); if missing, it says how to install it (`npm install -g @github/copilot`).

### Changed

- **Laser button**: the dot is no longer stretched like a pill and, when active, uses the theme's laser color.
- **e2e tests 6 times faster (`syna-7qa9`)**: each file waited an extra 30 s because Vite was looked up on 127.0.0.1 and Vite listens on ::1. It now starts in ~2 s, Vite is reused between files and shuts down by itself at the end; the full suite goes from ~34 min to ~5.5 min (160/160). `test:e2e:quick` runs everything in one process. If the WebDriver goes down, all its processes are closed (before, WebKitWebDriver was left orphaned).
- **CI and release**: a new workflow (`ci.yml`) checks types and runs the frontend and Rust unit tests on every push to `main` and on PRs. The release builds Linux, Windows and macOS in parallel (before, in series) and publishes the release at once at the end, with `latest.json` assembled with the platforms that built; if Mac fails it is published without it. The Rust cache is warmed from `main` (when `Cargo.lock` changes and every Monday): the one each tag saved could not be used by the next one, so every release compiled all dependencies from scratch. Tag validation also checks types and unit tests. The e2e tests remain local.
- **e2e tests without fixed pauses (`syna-7qa9`)**: `diagram-pan-zoom`, `compare-diagrams`, `compare-files-git`, `compare-images`, `explorer-keyboard-focus`, `editor-tab`, `document-words` and `drawio` wait for the real condition (diagram still, zoom applied, tree selection, diff text, PNG rewritten) instead of fixed pauses. New `bun run test:e2e:quick [name…]`: smoke plus the touched or named e2e tests.

## [1.0.0-beta.3] - 2026-11-19

### Added

- **Key outline in YAML and JSON**: the code editor for `.yaml`, `.yml` and `.json` has the same outline button as notes: keys in a tree (up to 4 levels; list items as `- value`, without the `$ref`s), with quick search, folding and the cursor's section highlighted. Choosing a key jumps the cursor to it. **Ctrl+F12** (configurable as `toggle_outline`, also in the palette) opens and closes the outline of the active tab (the key outline in YAML/JSON and the heading outline in notes) in a centered dialog that moves by dragging the header (double-click recenters it) and resizes from edges and corners, remembering its size; the editor button still opens the dropdown next to it. In both outlines, from the search box, ↑/↓ (and PageUp/PageDown) move through the visible entries, →/← expand and fold the active one and Enter goes to it.
- **Follow YAML and JSON `$ref`s**: in the code editor, `$ref` values (OpenAPI, JSON Schema) are underlined; with Ctrl+click or Ctrl+Enter the referenced file is opened (path relative to the current file) and it jumps to the pointer's key (`./paths/a.yaml#/post/responses`, also with `~1` in OpenAPI paths and list indexes). Internal pointers (`#/components/schemas/Error`) jump within the same file, and a broken reference warns instead of opening anything.
- **Git Log of all branches**: a dialog in the style of JetBrains' Git Log window with the commits of all the repository's branches and tags (`git log --all`), the branch graph in colors, message, author, date and hash columns, and labels for local branches, remote branches and tags on each commit. Selecting a commit shows its full message, author and changed files; double-click (or Enter) on a file opens its diff at that commit. Filter by branch or tag and by text (message, author, hash or branch). The HEAD commit is marked with a "HEAD" label (or "HEAD (detached)") and a hollow dot. Right-clicking a commit offers: checkout of its local branches, checkout of the commit (detached HEAD, with confirmation), new branch there (with or without checkout, like `checkout -b`), create tag (lightweight or annotated), delete its tags and local branches (with confirmation; an unmerged branch asks to confirm `git branch -D`) and copy the hash. It deletes nothing on the remote. It is opened from the command palette ("Git: Log of all branches") and from a repository folder's menu; with several repositories it has a selector.
- **Pixel-by-pixel diff between files of different types and between draw.io files**: "Compare files" and the git/history diffs offer "2 Panels" and "Differences" whenever both sides can be converted to an image, even if they are of different types (an image against an Excalidraw, Mermaid against D2, draw.io against Mermaid…). Two `.drawio` files are drawn with draw.io (hidden iframe, export to PNG) and compared, without losing the XML text diff. draw.io needs the URL from the settings: without a network it only works with a local draw.io. If one side cannot be drawn, the text diff remains as before.
- **Full-text search understands identifiers better and more formats**: `AchOperationCrService` or `sendTransferRequest` are found whole and also by their parts ("operation", "transfer request"); `snake_case` and `kebab-case` were already split. XML files (also `.xsd`, `.wsdl`, `.bpmn`) are indexed without tags, ids, coordinates or colors, instead of raw and truncated; Excel/OpenDocument spreadsheets by sheets and rows; and JSON/YAML with each value under its key path (`info.title: …`). The index is rebuilt once.
- **MCP analysis cache**: an ArchiMate model is traversed once and later queries (`inspect_archimate_element`, `read_note` with `outline_only`) take it from memory (from ~14 ms to ~0.3 ms with a 3.2 MB model). It invalidates itself if the file changes, stores only expensive analyses and has a 64 MB limit with least-recently-used eviction.
- **ArchiMate models (`.archimate`) in the MCP**: `read_note` with `mode: "outline_only"` returns the model's outline (folders and views with their line), and the new tool `inspect_archimate_element` looks up an element by id or name and returns in a single response its type, documentation, properties (e.g. `UNIQUECODE`), incoming and outgoing relations with the name of the other end, and the views where it appears. Full-text search indexes their names, documentation and properties (before, the raw XML was truncated and they were lost).
- **Structure of draw.io diagrams in the MCP**: `extract_note_diagrams` on a `.drawio`, `.drawio.png` or `.drawio.svg` (also with `revision`) returns per page the containers with their shapes, the loose shapes and the connections (from → to, with their text); `read_note` returns the diagram's text, and on an image it tells you to use `render_diagram`, which now also returns images (.png, .jpg, .svg…).
- **Filters in MCP searches**: `search_notes` and `full_text_search` accept `extensions` (e.g. `["archimate"]`); by default `search_notes` still searches only Markdown. `search_in_note` accepts `max_matches`.
- **Readable date in `feedback.md`**: each entry carries the date and time in addition to the unix date.
- **Several Git repositories in one vault**: subfolders that are repositories show their branch next to the name (with a dot if they have changes), and their files have git status, diff and history like those at the root. The explorer header shows the branch of the active file's repository. The commit and branch dialogs have a repository selector, and the repository folder's menu offers Branches, Commit and Push for that repository. A commit never mixes repositories. The git MCP tools (`get_vault_git_status`, `git_file_history`, `git_diff_revisions`, `git_read_revision`) use the closest repository that contains the file. Repositories are queried in parallel and the list is reused for a few seconds.

- **OpenAPI/Swagger specification viewer with Scalar**: `.yaml`, `.yml` and `.json` files whose root key is `openapi` (3.x) or `swagger` (2.0) open in Reading as browsable documentation, and in Edit with the code editor. It takes the colors of the app's active theme (also when switching between light and dark) and reloads when the file changes on disk. Scalar is bundled and loads only when one of these files is opened: no CDN, external fonts or telemetry.

- **YAML and JSON syntax highlighting** in the code editor for `.yaml`, `.yml` and `.json` (before they were colored as Markdown).
- **The release also builds for macOS** (Apple Silicon and Intel) in a separate job that runs after publishing: it uploads signed `synapsers-darwin-*.app.tar.gz` files for the updater and adds the platforms to `latest.json`. Without an Apple signature, macOS asks you to open it with right-click → Open.
- **`.xml` and `.archimate` files (Archi models) are visible in the vault explorer**, are indexed in full-text search and the agent can read and search them like the rest of the code.

### Changed

- **Tables in the Markdown editor (Live mode)**: table lines are no longer replaced by a widget: they remain document text and are painted as a table (borders, header, alternating rows, faint `|`), with the columns aligned by measuring the real width of the text (works with proportional fonts and with links inside cells). This way the numbering of the lines below no longer goes out of sync, and with ↑/↓ the cursor enters the table row by row instead of skipping it entirely. The cell editor is still available with the "Visual editor" button that appears when hovering over the header.
- **On Linux, the interface uses WebKitGTK's fast rendering (DMABUF)**: draw.io and the rest of the app run more smoothly. If with some driver the window comes out blank or flickers, `SYNAPSE_DMABUF=0` goes back to the previous mode.

### Fixed

- **Focus when reopening a vault**: the active tab restored in Edit mode receives focus as soon as its content loads, so you can type and move around the code (CodeMirror) without clicking first.
- **A PNG image saved from draw.io comes out sharp**: it is exported at scale 2 instead of 1, which looked pixelated when enlarged.
- **Text edited in Excalidraw with zoom no longer looks blurry**: the edit box is enlarged with its real font size and not with a scale, which WebKitGTK (with DMABUF) drew at low resolution and stretched.
- **"Diff" compared against HEAD instead of the stage**: like `git diff`, it now compares the local version with what is staged, also in images and diagrams (2 Panels and Differences), and the "before" side is called "Stage / Index". If all changes are already staged, it no longer silently shows the diff against HEAD: it warns "No unstaged changes" with a button to open "Diff --staged".
- **`full_text_search` with `folder_path` returned an empty `hits` with `total_matches` > 0**: the folder filter was applied after trimming the best results from the whole vault; now more candidates are requested and `total_matches` counts only those in the requested folder or extensions.
- **The attachment context asked to read images with `read_note`**: now, if an image or a diagram is attached, it points to `render_diagram` to view it and `extract_note_diagrams` for the structure of a draw.io.
- **The full-text index is rebuilt once** (version 4) to re-process the `.drawio` and `.archimate` files already indexed with the new extractors.
- **Code files (xml, json, archimate, …) can be edited when switching to Edit**: the editor stayed read-only even though the button said "Edit".
- **In Reading mode, code files and diagrams can be searched with Ctrl+F**: the editor now receives focus (cursor and selection) even though it does not allow typing.
- **When saving in draw.io, the other tabs with the image reload without losing zoom or position**; nor when leaving the editor.

## [1.0.0-beta.2] - 2026-11-17

### Added

- **`.xml` files are visible in the vault explorer**, are indexed in full-text search and the agent can read and search them like the rest of the code.

- **Tab copies with their own mode and a single editor per drawing**: each copy of a tab (`Duplicate tab`) has its own Reading/Edit button, so a note can be in edit mode in one tab and in reading mode in the other. Drawings (Excalidraw and draw.io) are only edited in one tab at a time, so they do not overwrite each other when saving: switching to edit in another copy takes you to the tab that is already editing it; when it is closed or switched to reading, any copy can edit. Copies of a draw.io image are updated with each save in the editor.
- **"Open in another tab (read-only)"** in the explorer's context menu, when the file is already open: opens a reading copy next to the original.
- **Drag to another window also the tabs of a version, a diff and copies** (before, only those of files).
- **Full-text search finds the text of draw.io diagrams**: `.drawio` files and PNG/SVG images with a diagram inside are indexed with the texts of their shapes and the names of their pages (also compressed diagrams), as documents of type `drawio`. Images without a diagram are still not indexed; to know this, only the PNG headers are read.
- **draw.io images are marked when indexing**: the vault index stores whether each PNG/SVG carries a diagram, so the Reading/Edit button appears when opening it without reading the file again. The cache moves to `.synapse/cache-v2.bin`; the previous one (`cache.bin`) is imported automatically the first time, without losing open tabs.
- **draw.io images: Reading/Edit button and content-based detection**: on a draw.io image, the header's Reading/Edit button opens the draw.io editor (Edit) or goes back to the image (Reading), as with notes; the viewer's button now says "Open in draw.io". A PNG counts as a draw.io one by its name (`.drawio.png`) or because it carries the diagram inside (the `mxfile` block that draw.io saves when exporting with "include a copy of the diagram"), even if it is called `photo.png`. The same with SVGs: a `.drawio.svg`, or any SVG with the diagram in its `content` attribute, is edited in draw.io and saved as SVG (`xmlsvg` format).
- **Review the vaults that can talk to this one**: in Settings → AI Agents you can see which vaults were accepted (✅) or rejected (🚫) and remove each one; the next time it is asked again.
- **The debug log includes the channel between Synapses**: besides ACP messages, it records the `/peer` requests that go out and come in (`peer-out`, `peer-reply`, `peer-in`, `peer-answer`), without the `ping`s.
- **Documentation of communication between Synapses** (`docs/comunicacion_entre_synapses.md`): who writes to whom, the `/peer` channel, permissions and limits.
- **Mention conversations and vaults in the agent chat**: `#` suggests the other conversations in this window and `@` also the other Synapse vaults (in addition to files). Each conversation is shown with its status (🧠 working, ⏸ idle, ⚠ waiting for a permission), title, agent and last activity; the tooltip carries its last reply and the last question. Vaults are marked 🟢 open, 🟡 busy or ⚪ off; `@Vault#` picks one of its conversations or `#new`. Choosing an off vault opens it in another window and the message waits for it to respond. The number is inserted (`#2`, `@Vault#2`) and, when sending, Synapse explains to the agent what each mention points to; in the sent message it appears as a chip with its title in the tooltip.
- **Filters in `@` mentions**: `@agent:` (or `@a:`) shows only agents, that is other Synapse vaults and this window's conversations, and `@file:` (or `@f:`) only files; the prefix only filters and when choosing, the mention is inserted without it. Without a prefix, if what is typed is the beginning of a vault name (`@main` → MAINFRAME), vaults appear first; when files and vaults are shown together, each group indicates its shortcut.
- **Default agent**: in Settings → AI Agents you choose an agent for all vaults. It is used in a vault that has not used any yet (for example, when another vault writes to it for the first time) and in "＋ New conversation"; if none is chosen or installed, the first in the list is used and downloaded automatically. Before, a new vault could not handle conversations from another.
- **Synapse asks for attention when it is waiting for your response**: with an agent permission or a question (for example, another vault that wants to talk), if the window is not in front it is marked as urgent in the taskbar and a system notification appears. It does not take focus away from what you are doing.
- **Messages from another agent arrive collapsed**: "From Agent · #2 · 2.3 KB ▸"; when opened they are shown rendered (not raw), with a button to copy the text as it arrived. Your agent still receives them in full.
- **ACP debug log**: in Settings → AI Agents, it saves unfiltered every message that goes in and out of the agents to `acp-debug.jsonl` in the configuration folder, to see what each agent sends (for example, whether it includes the result of its tools).
- **Alt+2 opens and closes the agent panel**: new action "Show / hide the agent" (`toggle_agent`) in the configurable shortcuts, which also appears in the command palette, in the shortcuts panel and in the tooltip of the sidebar button.
- **Copy and paste files with other applications**: "Copy" in the explorer puts the files on the system clipboard, like a file manager, so they can be pasted into another Synapse instance, the file manager or another application; "Paste" brings what was copied outside (another instance, the file manager) into the chosen folder ("Paste (N files from outside)").
- **Drag files from the explorer to other applications**: dropping them into Telegram, the file manager or another application attaches or copies them, as from a file manager; inside the tree, dragging still moves.
- **A file's Git History draws its branches**: a column on the left shows the graph (like `git log --graph`), with the merges that join branches that also touched the file. Each commit is marked "new" (created the file: hollow dot and no "View changes", because there is no previous version on its branch), "deleted", "renamed", "merge" or "other branch" (it is not the previous version of the commit above).
- **The agent understands a file's Git history**: new MCP tools `git_file_history` (the file's commits, following renames), `git_diff_revisions` (what changed between two commits, or against the local version, with added and removed lines) and `git_read_revision` (the file as it was at a commit); `render_diagram` accepts `revision` to view a diagram or an image (e.g. a `.drawio.png`) as it was. In Git History, with two commits selected (or just one, to ask about that commit), "Ask the agent what changed" opens the panel with the file attached and the question written.
- **Selecting commits in Git History**: click selects a commit, `Ctrl`+click adds or removes others and `Shift`+click selects the range from the last click. With right-click: "Diff between these 2 versions" (with exactly two selected) and copy their hashes, full or short, comma-separated.
- **Duplicate tab**: in each tab's menu, "Duplicate tab" opens another tab with the same content, next to the original and in the same pane; each copy has its own view (diff mode, zoom, scroll).
- **The diff viewer remembers views already computed**: "2 Panels", "Differences" (pixel by pixel) and text are computed the first time you choose them and stay in memory until the tab is closed, so switching between them is immediate. Hidden views keep their size and their already drawn images, so "2 Panels" is not seen loading again when you return. Each diff tab has its own mode: with "Duplicate tab" you can see "Differences" in one tab and "2 Panels" in the other; the last mode chosen is the initial one for new diffs.
- **`Shift+Escape` closes the last side panel you opened**: the explorer (or another left-bar panel) or the agent's, whichever you opened most recently; if you press it again, it closes the previous one.
- **Showing the explorer with the keyboard gives it focus**: `Alt+1` / `Ctrl+B` (in addition to "locate", `Ctrl+Alt+1`) leave the focus on the tree, on the open file or the first one, to move with the arrows and open with Enter.
- **Pinned or unpinned explorer**: a pin in the explorer header. Pinned (as always) it takes up its space next to the editor; unpinned, the editor uses the whole width and the explorer floats on top, attached to the sidebar, when you open it from there. It hides when opening a file, with Esc or on clicking outside. The choice is remembered.
- **Diff between two versions of a file**: in Git History, "Diff between versions" replaces each commit's actions with checkboxes; you choose exactly two and "Compare" opens their differences (from oldest to newest) in the usual viewers: text, images and diagrams. When turned off, the actions come back.
- **Git dialogs can be moved**: the history (log), commit and branches dialogs move by dragging their header, without leaving the window; they remember the position while the app is open and a double-click on the header recenters them.
- **AI agent in a side panel (ACP)**: new "AI Agent" button in the sidebar and in the editor header (next to the window controls) that opens a panel docked to the right or floating (movable and resizable). It converses with ACP (Agent Client Protocol) compatible agents about the open vault: streaming responses, tools the agent uses and permission requests with buttons to approve or reject. The verified agents are **Claude**, which is downloaded from the public ACP registry (the same one Zed uses) and verified with its sha256, and **Google Antigravity**, which uses the already installed `agy` CLI; you can also register your own agents (command, arguments and environment variables) in Settings → AI Agents.
- **The agent works with your vault**: each session receives the open Synapse window's MCP as `synapse_instance` (notes, search, links, Git…), which turns on by itself if it was off; agents that do not speak MCP over HTTP reach the same instance through a bridge (`synapsers --mcp-proxy`). At the start the agent is asked to use that MCP and to create and edit notes with it instead of writing the files. In builds with the `develop` feature there is also a **feedback mode** (Settings → AI Agents): the agent records in `~/.config/synapsers/feedback.md` what the MCP is missing, with the agent, the model and the conversation it came from. In addition, `/feedback [instructions]` in the message asks the agent to review the conversation and record with `report_feedback` what to improve in the MCP and the panel. In the message you can attach notes with `@` (`@this` attaches the file of the tab you are viewing), drag them from the explorer or send the editor selection with `Ctrl+Alt+A`; the open note is attached automatically as a tag you can remove. The panel shows the agent's plan, the files it touches with their diff, the commands it offers (with `/`), context usage and, if the agent has them, mode and model selectors (for example, Antigravity's Gemini and Claude models).
- **An external MCP client can talk to the panel's agents**: for example, Claude Code using Synapse's MCP. `send_to_conversation` works without a conversation of its own (the message arrives marked "External MCP client") and, if it was a question, the reply is collected with the new tool `read_replies`.
- **`/usage` in the agent panel**: answered by Synapse with a summary of the conversation (agent, model, mode, context, cost, messages, tools used by type, compactions and duration), unless the agent has its own `/usage`.
- **Conversations per process, per agent**: in Settings → AI Agents you choose how many conversations share one process of each agent (default 4; use 1 for an agent that handles one at a time).
- **Own octopus icon**: a robot octopus in SVG, which follows the theme color, marks the conversations and messages that come from another vault.
- **Agents converse with those of another vault open in another window** (🐙): "ask mainframe…" reaches the other Synapse window through a local channel with a token, and there it opens a new, clean conversation with its agent (or continues the thread of the same originating conversation, or uses `mainframe#1` if requested). The first time, the destination vault asks whether it accepts conversations from the other. The reply returns on its own to whoever asked. Conversations opened by another vault and their messages carry an octopus. If the destination vault is among your recents but not open, Synapse opens it in another window. In Settings → AI Agents you choose which agent handles those conversations; in them, the agent's reads are allowed automatically and everything else is rejected if nobody responds within 2 minutes. `render_diagram` also draws diagrams from another vault.
- **Agents query other vaults**: with `list_vaults` and the `vault` parameter in the MCP's read tools (`search_notes`, `read_note`…), one vault's agent can search and read another's notes (for example, validate the diagrams against the mainframe documentation), whether it is open or not. It is read-only; the first time Synapse asks whether that vault may query the other and remembers it (`.synapse/vault-access.json`). The reply is marked with 📚 and the vault name.
- **Agents talk to each other between conversations**: each conversation in the panel has a number (#1, #2…). If you ask an agent "ask #2…" or "notify Gemini…", it uses the new MCP tools `list_conversations` and `send_to_conversation`, even if they are different agents (Claude, Gemini…): Synapse delivers the message, and if the target is working it receives it when its turn ends (✉ on its row). If it was a question, the reply returns on its own to whoever asked ("waiting for #2" in the meantime). Messages from another agent are shown marked with their origin, and there is a limit of 3 consecutive round trips without user intervention.
- **More compact agent panel**: model and mode move to a ▾ next to the context line, in short form ("Argon·L", and the mode only if it is not the initial one; a model Synapse does not recognize is shown with its full name), with the full names in the tooltip and in the menu; the context line shows only the percentage (the detail goes in the tooltip). "MCP Synapse" and ⋯ move to the conversation title, next to the 📌, and the conversation bar disappears.
- **`/compact` in the agent panel**: if the agent offers `/compact`, the context usage line shows "Compact" (from 60% or on mouse hover, and in amber from 80%). On compacting, the chat marks "— context compacted —" and the bar drops with the next usage data. When resuming with an agent that cannot load sessions, the replacement summary starts at the last compaction. "View raw" and "Delete conversation" move to a ⋯ menu in the conversation bar.
- **Each vault remembers its agent panel**: whether the panel was open or closed, the last agent and the open conversations (expanded and pinned). When returning to that vault they are resumed with their messages. A vault that never used agents starts with the panel closed and no agent.
- **Several conversations with agents at once, in an accordion**: "New conversation with…" opens another at the top of the panel without closing the others (of the same agent or another); at most two stay expanded and, when expanding a third, the one that has been open longest is folded. Each row shows its status (🧠 thinking, "permission" if waiting for your response) and is marked if it finished while folded. 📌 pins a conversation to the side of the panel, in its own column; ✕ closes it and moves it to history, from where it is reopened. While the agent thinks, the chat shows it with 🧠, and the collapsed reasoning is just a 🧠 that expands on click. Conversations with the same agent share its process (up to 4 per process; with more, another is launched), so opening a new one is almost immediate; the process shuts down 30 s after its last conversation is closed. Pasted images are shown enlarged when clicking their thumbnail (Esc, click outside or ✕ to close).
- **Diagrams as images through the MCP**: when reading a diagram (Excalidraw, Mermaid, D2, PlantUML), `read_note` marks it with `diagram.render_tool`, and the new tool `render_diagram` returns it drawn as a PNG, just as it looks in Synapse (the open window draws it). This way the agent can see the diagram and not just its code.
- **Pasting images into the message to the agent**: `Ctrl+V` with an image on the clipboard (for example, a screenshot) attaches it with its thumbnail; if the agent does not accept images, it is told there was one. Each agent remembers the last model chosen, so a new conversation starts with it.
- **Agent conversation history**: each conversation is saved in the vault (`.synapse/agent-sessions/`) and resumed from the panel's history dropdown; each conversation remembers its agent and resuming it switches to that agent; when choosing an agent its last conversation is resumed (the panel starts closed and with no agent chosen); if the agent can load sessions (`session/load`) it continues with its own context, and if not, it receives a summary of what was said. There are buttons to start a new conversation, open the conversation raw (JSON) with the default application and delete one conversation or all. Below the message you can see the model's context window usage (tokens used, total and percentage, and the cost if the agent reports it). The agent's replies are shown as markdown (links to vault notes open them in Synapse) and each tool expands to show its input and output. The panel header aligns with the tab bar and the docked panel follows the Islands theme.
- **Wider, resizable Git History**: the dialog is enlarged by dragging its edges or corners, like a window, and remembers the size for all files. The two diff buttons stack one above the other and the commit message takes up to two lines. Commits with a tag show the tag name as a label next to the message.
- **Larger, resizable commit and branches dialogs**: they open wider and taller, are enlarged by dragging their edges or corners and remember their size; the file or branch list uses the extra height. The three Git dialogs (commit, branches and history) now share the same frame, header and footer.
- **View a commit's changes**: in Git History, "View changes" opens that commit's patch for the file (against its parent commit). Text in Split, Unified or Patch; images, drawings and diagrams with 2 Panels and Differences.
- **Pixel-by-pixel diff of diagrams**: Mermaid, D2 and PlantUML have the **Differences** mode in the Git diff, in the stage, in a commit's patch and when comparing two files. The two versions are rendered and compared in memory, without writing anything to disk.
- **Compare two drawings or diagrams**: "Compare files" with two Excalidraw files or two diagrams of the same language offers 2 Panels, Text and Differences.
- **Compare two images**: "Compare files" with two images (or two drawings, diagrams or PDFs of the same type) shows the visual mode: **2 Panels** side by side and, for raster images and Excalidraw drawings, pixel-by-pixel **Differences**. If the images have different sizes, they are compared from the top-left corner.
- **Complete tab menu**: besides open in a new window, with the default app and show in folder, it now allows **Rename file** (opens editing in that tab's breadcrumb), **Copy relative path** and **Copy full path**, just like the breadcrumb menu.
- **Embedded draw.io editor**: `.drawio` files open in the draw.io editor inside a tab and on saving the diagram's XML is written. `.drawio.png` files are still viewed as an image and edited with **Open with draw.io** in the explorer's context menu or with the viewer's **draw.io** button; on saving, the PNG is rewritten with the embedded diagram, which remains editable. It respects the light or dark theme. It is loaded from `embed.diagrams.net` (the diagram does not leave the machine: it travels via `postMessage`); the address can be changed with `drawioUrl` in the settings to use your own draw.io without a network.
- **`.drawio.png` files open as an image again**: draw.io mode lasts only while the tab is open; opening the file from the explorer or quick open, or closing the tab, returns it to the image viewer.

### Changed

- **Faster tests**: the Rust integration tests are built into a single binary (`src-tauri/tests/integration/`; a group is run with `cargo test --test integration <module>::`), the `dev` and `test` profiles keep less debug information, and linking uses `mold`. Recompiling the tests after a change goes from minutes to a few seconds. The e2e tests use the same development binary (`target/debug`, Vite on 5173) instead of a separate build in `target/e2e`.
- **"Vault" instead of "Workspace"**: the welcome screen, recents, shortcuts and palette say "vault" (in Spanish, "bóveda"), like the rest of the app.

### Fixed

- **Publishing releases builds on Linux again**: the release workflow installs `mold`, the linker used by `src-tauri/.cargo/config.toml` (it was missing on the runner and failed with "cannot find ld").
- **Clicking in the explorer on an image open in draw.io no longer closes or redraws the editor**: it only activates its tab.
- **An open image refreshes when the file changes on disk**, including in its tab copies; before, the webview kept showing the cached version.
- **Saving an image (PNG/SVG) in draw.io no longer redraws the editor**: the file rewritten by its own save was taken as an external change and the editor was mounted again. Now, while the image is in the editor, a change on disk does not reload it, and when leaving the editor the viewer shows what was saved.
- **More robust e2e tests (in progress, `syna-7qa9`)**: if the WebDriver goes down, the remaining tests fail instantly instead of waiting for their timeout; startup waits for the previous WebDriver to release port 4444 (probable cause of the chained crashes with ECONNREFUSED); several tests replace fixed pauses with waits for the real condition.
- **Shift+F6 (change all occurrences) did nothing if the window did not have system focus**: the shortcut checked `hasFocus`, which in WebKit depends on `document.hasFocus()`; now it is enough for the key to reach the editor.
- **Tabs of a version or a diff in a new window**: "Open in new window", "Move to new window" and the header button now also work with a commit's "View file" tab and with diff tabs (before, only with files); the new window shows the same version.
- **Default background of the empty tab was blank**: the light and dark backgrounds came in AVIF, which the WebView on some Linux systems (WebKitGTK without AVIF support) cannot decode; so it appeared blank when opening the app and when removing a custom background. Now they are WebP.
- **"Rename file" from a tab's menu**: the name editing no longer closes by itself. When the tab was activated, its reading view took the focus, the name field lost it and the edit was confirmed unchanged; now the reading view does not take the focus away from a text field, and the menu does not return the focus to the tab when choosing rename.
- **Git history of renamed files**: "View file", "View changes", "Diff with local" and the diff between versions of a commit prior to a rename or to moving the file to another folder now show their content, instead of "The file did not exist in the revision". In that revision the name it had at the time is looked up, just as the history follows the file.
- **Zoom and dragging of diagrams and images**: in diagrams (Mermaid, D2, PlantUML) dragging now moves the diagram in step with the mouse at any zoom (before, with zoom, it drifted), and the wheel zooms anchored at the cursor. The zoom of the image diff and of the Excalidraw viewer is also now anchored at the cursor, as the image viewer already did.
- **`bun run check` without errors or warnings**: the type of the pixels in the Excalidraw diff and the title schema, which deliberately took only the initial value.
- **Opening a conversation while the vault restores its own no longer folds it** when the restoration finishes.
- **The agent no longer stays in the previous vault when switching vaults**: the conversation is closed (it stays saved in its vault) and the panel starts over with no agent chosen, with the history of the new vault.
- **`Shift+F6` selects all occurrences in the code editor again**: there were two copies of CodeMirror and the shortcut was registered in one that the editor did not read; now a single version is used.
- **The vault cache is no longer corrupted if the app is closed while saving**: `.synapse/cache.bin` is saved with `object_store`, which writes to a temporary file and renames it when done; if the save is interrupted, the previous version remains.
- **Going back to reading keeps the position**: when switching from editing to reading in a Markdown note, reading opens at the part of the document that was being edited instead of going back to the start.
- **Visual diff on stage**: it compared the disk against the stage; now it compares the stage against HEAD, like `git diff --staged`.
- **Git History dates**: they showed "Invalid Date".
- **PlantUML in the side-by-side diff**: the Git version panel drew it as Mermaid.
- **"Diff" with two files selected**: in a vault with Git, the explorer menu's "Diff" did the Git diff of a single file (and often showed "No differences"). Now, with two files selected, it is called "Diff between the 2 files" and compares them with each other.

## [1.0.0-beta.1] - 2026-11-12

### Added

- **Built-in "Papyrus" theme**: aged kraft paper with fibers and stains, the note as a sheet with torn and toasted edges, and sepia ink with serif headings. In dark, tanned leather with parchment lettering. Also in "Papyrus Islands".
- **Global theme folders**: the `.css` files in `~/.config/synapsers/themes` (which is always read) and in the extra folders you add in Settings → Appearance show up as themes in all vaults, without recompiling. The extra folders are saved in `appearance.theme_dirs` of the new `~/.config/synapsers/config.jsonc`; they are read at startup and with the **Rescan** button. They support `@nombre`, `@capa` and `@base` like the built-in ones.
- **Built-in "Neon CRT" theme and automatic built-in themes**: "Neon CRT" is Neon with old-monitor-style scan lines in reading view (on Linux they can lag when scrolling, which is why "Neon" no longer includes them). To add a built-in theme, just drop its `.css` in `src/entities/appearance/themes/`, with `@nombre`, `@capa` or `@base` in its initial comment.
- **Automatic updater (Auto-updater)**:
  - Integration of the `tauri-plugin-updater` plugin to detect and apply signed updates from GitHub Releases.
  - Cryptographic digital signing of the packages and automatic generation of the `latest.json` manifest in the CI/CD workflows for Linux (`.tar.gz`) and Windows (`.zip`).

### Changed

- **A single global configuration folder, `~/.config/synapsers`**: the MCP port registry (`mcp-ports.json`) is no longer also written to `~/.config/synapse`. That folder is no longer used and can be deleted.
- **Navigation cache moved to `postcard`**:
  - Replaced the unmaintained `bincode` library with `postcard` (`v1.1` with `use-std`) for the binary on-disk serialization of the navigation engine cache (`DashMap<NoteId, NoteMeta>`), reducing footprint and ensuring modern type compatibility.
- **Build optimization**:
  - Compilation with *ThinLTO* in release mode and use of the `lld` linker on Linux to speed up linking and packaging times in CI.

## [1.0.0-alpha.5] - 2026-11-11

### Added

- **Presentation web page** (`docs/index.html`): static landing page based on `docs/about.md`, with enlargeable screenshots (viewer with full-size zoom and arrow-key navigation), comparison, themes, architecture and the mascot (kawaii octopus) fixed in the bottom right corner.

- **Built-in "Rose" and "Islands" themes**: "Rose" in soft pink tones; "Islands" shows the panels as floating islands with rounded corners on a darker frame, and combines with the others in "Paper Islands", "Neon Islands" and "Rose Islands".
- **Built-in "Paper" and "Neon" themes**: they come with the app and appear in Settings → Appearance → Custom theme without copying files. A file in `.synapse/themes/` with the same name replaces them. "Neon" is cyberpunk with a grid background, translucent panels with blur, headings with a magenta glow, cyan and yellow, its own folder icons and hand-drawn dropdowns. It has a dark version and a light "dawn" one.

- **Themes: all color in variables, public zones and example theme**: the interface colors come from CSS variables (new: `--success`, `--warning`, `--danger`, `--purple`, `--on-accent`, `--overlay-bg`…), so a theme changes the whole app. The main zones have stable `synapse-*` classes (sidebar, explorer rows, tabs, editor, reading, status bar, dialogs). Guide in `docs/temas.md` and example theme "Paper". Some tones change slightly by following the theme variables, especially on the welcome screen.

- **Per-vault themes and CSS snippets**: in Settings → Appearance you choose a theme from `.synapse/themes/` and activate snippets from `.synapse/snippets/` (`.css` files). They apply without restarting and each vault stores its own in `.synapse/appearance.json`. Themes can change the headings per level in reading and editing (`--h1-font`, `--h1-size`, `--h1-weight`, `--h1-color`… up to `h6`), the font of the notes' text (`--font-text`) and the folder icon (`--icon-folder`, `--icon-folder-open`).

- **Pixel-based Excalidraw diff**: the visual diff of a drawing has the "Differences" mode, like images. Both versions are exported to an image with the same frame and the pixels that changed are highlighted in magenta.

- Default background in the empty tab: one image for the light theme and another for the dark one, included in the app. They are used until a custom background is chosen.
- The empty tab's custom background is now per vault, not global. The one already chosen moves to the open vault.

- **Explorer: expand and collapse all**: two buttons in the explorer header. With the Git filter active they only affect the folders in the filter; when it is removed, the others stay as they were.
- **Explorer: deleted files**: a deleted file that Git still has keeps appearing in the tree, in red and struck through; clicking it opens its diff.
- **Complete Git filter**: with the Git filter the explorer shows everything that would go into a commit (untracked, modified, added or deleted), even if it is a file type that is normally not shown in the explorer.
- **Note outline**: clicking the title of a Markdown note (or the outline button next to it) opens an outline with all its headings and subheadings, collapsed by default (only first-level ones are visible), with buttons to expand and collapse all; choosing one takes the note to that heading. The outline opens expanded down to the heading where you are, which is highlighted, and has a quick heading search at the top (Enter goes to the first match).
- **Location in the note**: next to the title of a Markdown note, in parentheses, you see the heading path of the part you are viewing (e.g. `(Inicio › Parte A › Detalle)`), which updates as you scroll; clicking a segment goes to that heading.
- **From reading to editing without losing your place**: when switching from reading mode to an editing mode, the cursor is left on the first line that was being viewed in reading.
- **Breadcrumb: copy full path**: the file's context menu in the breadcrumb offers "Copy relative path" and "Copy full path".

### Fixed

- **Aligned headers**: the bottom line of the explorer header now matches that of the tab bar.
- **Code blocks in reading**: inside a code block a horizontal line appeared under each line of text (it inherited the inline code border).
- **Commit with deleted files**: committing a deleted file whose deletion was already staged (or a whole deleted folder) failed with "git add failed: … did not match any files".

## [1.0.0-alpha.4] - 2026-11-09

### Added

- **Settings panel**: the ribbon's gear button opens a panel to change the theme, the language, the text size, the empty tab background, Vim mode, the note properties, the Mermaid and Excalidraw viewers, the PlantUML server and all the keyboard shortcuts. The shortcuts are saved in `shortcuts.jsonc` without losing their comments.
- **Empty tab with search and background**: the empty tab shows the `Ctrl+O` search at the top, with the focus set to start typing, and lets you choose a background image with "Change background".
- **Abbreviation `hs:` for `headings:` in content search**, and the search guide now explains this filter.
- **`hs:` and `headings:` also work in Quick Open (`Ctrl+O`)**: they search in the headings without having to type `?` first.
- **When filtering by `headings:` or `hs:`, the result shows the heading found formatted as a title** and, on the next line, the text that follows it. The index is rebuilt only the first time the vault is opened after updating.
- **MCP: `write_note` reports what it did**: the response indicates with `created` whether the note was created or overwritten, and adds `warning` if its folder did not exist.
- **MCP: `write_note` no longer overwrites notes without permission** (breaking change): to replace an existing note you have to send `expected_hash` or `overwrite: true`; otherwise it fails and explains how to proceed. It also fails if `expected_hash` is sent and the note does not exist.
- **Text size in the editor and the preview**: `Ctrl+wheel`, `Ctrl++` and `Ctrl+-` change it, and `Ctrl+0` resets it. The shortcuts are configured in `shortcuts.jsonc` (`zoom_in`, `zoom_out`, `zoom_reset`) and support several combinations separated by commas.
- **Compare two files from VaultExplorer**: when selecting exactly two files, the context menu offers "Compare files" and opens a diff between them, with no need for git. It only supports text files.
- **Content search indexes the headings and subheadings of Markdown notes** and gives them more weight than the rest of the text: a note with the term in a heading comes before one that only mentions it in a paragraph. You can also filter with `headings:term`. The index is rebuilt only the first time the vault is opened after updating.
- **`kind:` filter in Quick Open (`Ctrl+O`)**: `kind:pdf report` searches only among PDFs; `kind:markdown`, `word`, `excel`, `code`, `image`, `mermaid` and `excalidraw` work the same way, just type the beginning (`kind:pd`). `kind:pdf` alone lists all PDFs.
- **`[[note#Heading]]` links go to the heading**, as in Obsidian: when clicked (in the editor and in reading mode) the note opens and goes to that heading or subheading, not to the first time the text appears. With `[[note#Heading#Subheading]]` the last one is used.
- **`![[note#Heading]]` embeds that section in reading mode**: it shows the heading and everything that belongs to it, up to the next heading of the same or higher level (lower-level subheadings are included). Embeds inside the embedded content are also expanded, up to 3 levels and without repeating cycles. If the note or the heading does not exist, it stays as a link to the note (before, a broken image was shown). In the editor it is still a link to the heading. Images and other resources with `![[...]]` do not change.
- **`Shift+F6` shortcut in VaultExplorer to rename**: it lets you rename the selected file or folder when the file explorer has the focus.

### Fixed

- **Content search no longer loses results when the last word is complete** and its stem is shorter, like "para" in `hs:Prompts para`.
- **`Tab` in the code and Markdown editors inserts 4 spaces** instead of taking the focus out of the editor; with selected text it indents the lines, and `Shift+Tab` removes the indentation. With Vim, only in insert mode.
- **Search in reading mode no longer freezes the app while typing** in large notes: it searches only once after you stop typing, and clearing the highlights is much faster.
- **`Ctrl+Shift+F` opens content search** also with a note open; before, it opened the search within the note.
- **`Ctrl+R` in an editor opens find and replace** with the focus in the replace field. With Vim it is still redo, except with the focus in the search panel. Outside the editor it still reloads.
- **Note properties recognize inline lists** like `tags: [uno, dos, tres]`.
- **`Shift+F6` in CodeMirror only activates if it has the focus**, avoiding capturing the combination or selecting occurrences when the editor is not focused.

- **The code editor's error messages are no longer cut off or covered by the top bar**, for example in an error on the first line of a diagram.
- **`Ctrl+R` in an editor with Vim redoes** (as in Vim) instead of reloading the file; outside Vim it still reloads.
- **D2 and Mermaid rendering no longer lags while editing**: before, every keystroke launched a render and, with a slow layout like TALA (~3 s per render), they queued up and the diagram took tens of seconds to catch up. Now it waits until you stop typing (longer if the render is slow) and there is never more than one pending.
- **D2 and Mermaid rendering while editing**: a render is no longer launched per keystroke (with TALA they piled up and the diagram arrived very late); it waits for a pause in typing and there is never more than one render queued. `Ctrl+Enter` forces the redraw and the status bar shows the render in progress and how long it took.
- **The diagram editor (D2/Mermaid) regains focus and the cursor when returning to its tab**, and a reload of the file from disk no longer recreates the editor or sends the cursor to the start: only the part that changed is applied.
- **Vim search (`/`) and cursor invisible in the dark theme** of the code editor.
- **A second instance of the app no longer rewrites the vault's MCP port** or marks it as stopped.
- **Editable title** of the note: it was saved onto a non-reactive property.
- **Quick Open**: consistent separators and stable order in ties.
- **Persistence of MCP state when closing and opening workspaces**:
  - The lifecycle of the runtime and ephemeral TCP socket (`running`) was decoupled from the preference configured by the user in the vault (`enabled`).
  - When closing the application, window or switching between vaults, the MCP server stops its process/socket without overwriting or turning off `enabled: false` in `.synapse/mcp.json`.
  - When opening a workspace where MCP was active, the server is reactivated automatically.
  - The domain trait `McpConfigRepository` and its infrastructure implementation `FileMcpConfigRepository` were introduced following the Onion Architecture.

### Changed

- **`F2` toggles between editing and reading** (before `Ctrl+E`).
- **In the editor, `Ctrl+E` and `Ctrl+Y` scroll the screen one line and `Ctrl+D` duplicates the line**, also with Vim (in Vim, `Ctrl+D` stops being half a page).
- **Performance**:
  - Faster startup: tabs are mounted when activated, the explorer no longer uses a fallback tree and the read commands are moved off the main thread (JS blocking at startup from ~1250 to ~750 ms).
  - The backend returns freed memory to the system after opening the vault (from ~800 to ~400 MB).
  - Virtualized explorer: only the visible rows are painted.
  - Images served with Tauri's asset protocol instead of base64 over IPC.
  - A single vault list in the frontend, with no IPC flood on opening.
  - Full-text search: querying the indexed versions no longer loads every document (19 ms → 3 ms with 3 740 documents).

### Added

- **PlantUML support (`.puml`, `.plantuml`)**: editor with highlighting and autocompletion, preview, errors marked on their line, `Ctrl+Enter` to redraw, creation from the explorer and the palette, and export to PNG/SVG. Rendering is done by a PlantUML server (by default `plantuml.com`, so the diagram code leaves your machine); it is changed with the "PlantUML: change server" command. The received SVG is sanitized before being shown.
- **`Ctrl+Enter` in the D2 and Mermaid editor forces the redraw** of the diagram.
- **Diagram render status in the status bar**: "Rendering D2… 1.2 s" while it lasts and the time it took when finished.
- **Autocompletion with the document's words** in all editors (markdown, D2 and Mermaid): the names you already use (variables, nodes…) are suggested even if the language does not know them, after the language's suggestions.
- **Export diagrams to PNG and SVG** from the explorer's context menu, for Excalidraw, Mermaid and D2. The file is saved next to the diagram with the current theme.
- **`Shift+F6` changes all occurrences of a word** (like renaming in JetBrains, but textual): it selects the exact whole-word occurrences and whatever you type changes them all. With Vim, they are changed with `c`.
- **Folding between braces in D2 and Mermaid**, from the code editor's gutter.
- **Vim folding shortcuts** with Vim's behavior: `zc`, `zo`, `za`, `zC`, `zO`, `zM` and `zR`. `zc` folds the block containing the cursor and `zM` folds all levels.
- **Copy the vault path** from the explorer's context menu; copy path is disabled if no files are selected.
- **Compatibility with the Obsidian plugin API**:
  - The facade responds synchronously as in Obsidian: `getAbstractFileByPath`, `getActiveFile` and `metadataCache.getFileCache`.
  - `SynapseVault` maintains a path → metadata index (without content) that is updated with local mutations and the watcher's events. Plugins are loaded when the index is ready.
  - What is specific to Excalidraw moves to `integrations/excalidraw`.

- **"Compatible only" filter in the community plugins catalog**:
  - Active by default: it shows only the manually verified plugins, listed in `verified-plugins.jsonc` (`verified_plugins`).
  - New file `verified-plugins.jsonc` at the project root, imported at build time with `?raw` and parsed in `community-plugins.svelte.ts`.
  - It can be disabled with the checkbox next to the sort selector.

## [1.0.0-alpha.3] - 2026-10-30

### Added

- **Undo and redo in the editor**:
  - `Ctrl+Z` to undo and `Ctrl+Shift+Z` or `Ctrl+Y` to redo.
  - Vim shortcuts: `u` to undo, `Ctrl+R` to redo and `U` to revert the last edited line.
  - Reusable module `shared/lib/editor-history`.

- **Comment/uncomment lines with `Ctrl+/`** (`Cmd+/` on macOS):
  - New module `shared/lib/editor-comment`, available in the Markdown editor and in the diagram code editor.
  - It uses the language's comment tokens: `#` for D2, `%%` for Mermaid and `<!-- -->` for Markdown.

- **Line undo (`U`) in the editor**:
  - New module `shared/lib/editor-history` that replicates vim's `U`: it reverts the changes of the last edited line, and reapplies them if repeated.
  - Available in the Markdown editor and in the diagram code editor.

- **Build version derived from Git**:
  - New module `version.rs` that composes the SemVer version from `git describe` (for example `1.0.0-alpha.2+4.g09ab330`), and adds the timestamp if the tree is dirty.
  - `build.rs` emits the information with `vergen`. The `get_app_version` command and the `--version` option use this version instead of `package_info()`.

- **Saving pending changes before file operations**:
  - New bridge `shared/lib/pending-writes`: the explorer, the breadcrumb and the Git menu persist the editor buffer before renaming or doing `git add`.
  - With this, renaming or staging an open file no longer operates on an outdated version on disk.

- **Resizable code panel in D2 and Mermaid diagrams**:
  - New `SplitResizer` bar between the code and the diagram viewer.
  - The chosen proportion is remembered per file during the session, in backend memory (commands `get_diagram_split_ratio` and `set_diagram_split_ratio`). It is not persisted to disk.

- **Renaming of Git-versioned files**:
  - If the file is in the index, the rename is prepared in the index (`git add` of the old and the new path), so that Git detects it as a rename.
  - New method `GitService::is_tracked` to check whether a path has versioned files.

- **Move files and folders in the explorer with `Ctrl+X` and `Ctrl+V`**:
  - New command `move_vault_items`: moves items to the destination folder, without overwriting or moving a folder into itself.
  - Open tabs follow their files after the move, and versioned files are staged in the Git index.
  - If an item is locked and has unsaved changes, confirmation is requested before moving it.

- **Keyboard shortcuts panel** (`F1` or the "Show keyboard shortcuts" command):
  - It lists the configurable shortcuts from `shortcuts.jsonc` and the editor's fixed ones, with search.
  - New `Kbd` component to display combinations, also used in the command palette.

- **Confirmation when renaming locked files**: if the locked file has unsaved changes, the pending changes dialog offers to save and continue, or cancel.

### Changed

- **Minimum width of the diagram code panel**: goes down from 280 to 160 px.

- **Shortcuts in the sidebar**: the Ribbon labels show only the main combination, via `primaryShortcutText`.

- **Autosave when removing the lock**: a file with pending changes when unlocked is automatically saved again.

- **Rust metadata and edition**: `Cargo.toml` moves to `edition = "2024"` and `rust-version = "1.93.0"`, with the project's author, description and repository. The `version` line is left commented out, so the version no longer comes from `Cargo.toml`.

- **Note editor performance**:
  - Autosave 4 s after you stop typing. It also saves on losing focus, on switching tabs or on switching to read-only, respecting locked files.
  - The whole vault is no longer reconciled on every save.
  - `git status` refresh grouped with a 5 s debounce.
  - Full-text indexer (FTS) debounce of 5 s.
  - New keys in `config.jsonc`: `editor.git_status_refresh_debounce_ms`, `indexing.debounce_ms` and `drawing_change_debounce_ms` (2500 ms by default).

- **Excalidraw**:
  - Change detection by element signature, so that scrolling, zooming and selection do not serialize the scene.
  - 2500 ms debounce and immediate propagation of pending changes on losing focus.
  - New key `drawing_max_wait_ms` (15000 ms): cap on continuous editing without pauses, after which the drawing is propagated and saved anyway.
  - The indexer skips reindexing if the drawing's text has not changed.

- **D2 and Mermaid diagrams**: the ```` ```d2 ```` and ```` ```mermaid ```` blocks are no longer re-rendered when editing nearby text.

### Fixed

- **`Ctrl+R` no longer reloads the application window**.
- **D2 SVG size**: the SVG gets `width` and `height` from its `viewBox`, preventing it from collapsing in reading cards.
- **Renaming or doing `git add` with unsaved changes**: the editor buffer is persisted before the operation, to avoid working with the outdated file on disk.
- **Tabs after renaming a folder or a file**: all open references are reassigned (main and split tabs, selection, history, closing, locks and recent paths), including notes inside a renamed folder.
- **Late save recreated the old file**: a pending save or a viewer that closed after a rename would recreate the old path. It no longer happens.
- **Drawings unsaved on losing focus**: when the window is hidden or loses focus, drawings with changes not yet propagated are also saved.
- **Renaming or moving a locked file without confirmation**: confirmation is now requested if the file is protected.

## [1.0.0-alpha.2] - 2026-10-30

### Added

- **Reusable `GitStatusBadge` component and CSS token support for Git**:
  - Creation of the `GitStatusBadge.svelte` component in the `@entities/git` entity to centralize the visual presentation of Git status badges (`M` modified/staged, `A` added, `?` untracked, `S` stashed).
  - Integration of the `GitStatusBadge` component in both `VaultExplorer` and `BreadCrumb`, unifying their design and interaction across the interface.
  - Addition of semantic CSS variables for Git states in the light and dark themes (`--git-modified`, `--git-staged`, `--git-untracked`, `--git-stashed` and their respective `-bg` and `-border` variants).

- **Smart image pasting in Markdown notes**:
  - Interception of paste events (`paste`) in CodeMirror when the clipboard contains an image (file or binary data).
  - Automatic detection of the corresponding image format (`webp`, `png`, `jpg`, `svg`, `gif`, etc., with a default fallback to `webp`).
  - Physical saving of the image in the `attachments/` subfolder relative to the current note's folder (or at the root if the note lives there), with a `image_YYYYMMDD_HHmmss.[ext]` timestamp and collision resolution.
  - Automatic insertion of the transcluded link `![[image_...]]` at the note's cursor position, rendering visually right away thanks to the editor's image support.

- **Dynamic application version and adaptive display**:
  - Exposure of the Tauri IPC command `get_app_version` in the backend to obtain the version reliably from `package_info().version`.
  - Connection of the dynamic version in the welcome screen (`WelcomeScreen`), replacing hard-coded values.
  - Version indicator integrated in the bottom status bar (`StatusBar`), with adaptive visibility through CSS styles that hide it automatically if there is not enough horizontal space.

- **Reorganization of tools in the Ribbon sidebar**:
  - Reordering of the tools, placing right below the folders the access to file search by name (`quick-open`) and full-text search in the notes' content (`search`).

- **Composite score in Quick Open (`Ctrl+O`)**:
  - New module `ranking.rs` (`QuickOpenScorer`) built on `nucleo_matcher`, which scores per field (file name, title, tags and path) with weights: name > title > tags > path.
  - Bounded recency bonus (last day, last week or opened at some point) and a bonus for an exact name match, so that a recent scattered match does not outrank a clean match.
  - Integration tests in `navigation_tests.rs` for the new result order.

- **YouTube video thumbnails in Markdown notes**:
  - Links to YouTube (`youtube.com`, `youtu.be`, `youtube-nocookie.com`, including `shorts`, `embed` and `live`) show a clickable thumbnail in reading mode and in the live editor.
  - New module `youtube.ts` with video ID extraction and unit tests in `youtube.test.ts`.
  - The thumbnail is hidden automatically if it cannot be loaded.

- **Title synchronization in the frontmatter**:
  - New function `sync_frontmatter_title` that updates or creates the `title` field in the YAML frontmatter of a Markdown note, preserving the rest of the fields and the body.

- **`.txt` extension treated as code**:
  - `.txt` files are added to the list of code extensions in `config.jsonc` and are shown with the code icon instead of the Markdown one.

### Changed

- **Autosave with a per-file timer**:
  - Each note has its own save timer, so editing one note no longer cancels another's pending save.
  - It saves immediately when the window loses focus, when it is hidden, when switching tabs or when switching to read-only, always respecting the file lock.
  - `auto_save_delay_ms` goes from 600 to 4000 ms and `fs_watcher_recent_save_grace_ms` from 3000 to 8000 ms.

- **Grouped Git refresh**: new key `git_status_refresh_debounce_ms` (5000 ms) in `config.jsonc`. Saves group the `git status` instead of launching one per save.

- **Configurable Full-Text reindexing**: new key `indexing.debounce_ms` (5000 ms by default, previously fixed at 350 ms). The whole vault is no longer rebuilt on every save: the watcher detects the change.

- **Vault explorer order**: folders with an emoji at the start come first, followed by the rest of the folders and then files, all in case-insensitive alphabetical order (`sortSiblings` in `VaultExplorer.svelte`).

- **D2 engine migrated to `@d2lang/d2`**:
  - The `@terrastruct/d2` package is replaced by `@d2lang/d2` (v0.1.34) in `package.json` and in the `d2-service.ts` service.
  - New `tala` layout engine available in the D2 selector alongside `dagre` and `elk`.

- **Quick Open sorting rewritten**: the tier system and manual sorting are replaced by `QuickOpenScorer`'s composite score, and the fallback linear scan only runs when nucleo does not finish in time or has not injected all the notes.

### Fixed

- **Quick Open word filtering**: each word of the query must appear in the field, in any order (for example, `bcp ttib` finds `BCP - TTIB ...`).
- **`nucleo.tick` loop in search**: the loop no longer runs an extra `tick` after nucleo's work finishes.
- **Mermaid diagrams that re-rendered while typing**: the widget no longer compares absolute positions, so typing before a diagram does not destroy it and render it again on every keystroke.
- **D2 diagrams with zero height in WebKit**: the generated SVG gets `width` and `height` derived from its `viewBox`, preventing it from collapsing inside the reading cards.

## [1.0.0-alpha.1] - 2026-10-28

### Added

- **Integration of the Tauri CLI plugin (`@tauri-apps/plugin-cli` / `tauri-plugin-cli`)**:
  - Addition of formal CLI support in the desktop application through the official Tauri v2 plugin.
  - Version query (`--version`, `-V`) and detailed help (`--help`, `-h`) options that print information directly in the terminal and end execution immediately without instantiating the graphical interface.
  - Documentation in `--help` of the vault/file opening options (`--vault`/`-v`, `--file`/`-f`, positional path) and of starting the local MCP stdio server (`--mcp`).

- **Preservation and reopening of closed tabs in history navigation**:
  - When closing an active or background tab, its record is kept in `tabHistory` in a decoupled, low-memory way.
  - When using back or forward navigation (including the mouse side buttons or <kbd>Alt+Left</kbd>/<kbd>Alt+Right</kbd>), if the target tab had been closed, it is reopened automatically, restoring its original physical position in the tab bar and preserving the cursor selection state.

- **Editor and Excalidraw timing parameters configurable in `config.jsonc`**:
  - New properties under the `"editor"` block, typed and validated both in the Rust domain model (`EditorConfig`) and in the frontend entity (`file-types.svelte.ts`):
    - `drawing_auto_save_delay_ms`: Idle time before persisting drawings to disk (3000 ms by default).
    - `drawing_change_debounce_ms`: Wait time to propagate scene changes to the tab (400 ms by default).
    - `fs_watcher_recent_save_grace_ms`: Grace window to ignore watcher events originating from our own saves (3000 ms by default).

- **Obsidian API compatibility layer: Global environment, Node.js emulation and CodeMirror stubs**:
  - **Module emulation in `require()`**: Support for the `@codemirror/search` and `@codemirror/autocomplete` packages, as well as essential Node.js modules required by community plugins (`path`, `events`, `constants`, `url`, `punycode`, `crypto` with `createHash` and `randomBytes`, `stream`, `process`, `util` and `querystring`), along with safe stubs for desktop dependencies (`electron`, `child_process`, `fs`, `http`, `https`, `os`).
  - **Native `Buffer` polyfill**: Implementation based on `Uint8Array` with binary UTF-8 and Base64 encoding and decoding (`Buffer.from`, `isBuffer`, `toString`).
  - **Global environment and canonical objects**: Exposure of `activeWindow`, `global`, `process` and `CodeMirror.defineMode` / `defineSimpleMode` in the global execution context.
  - **New exports in the Obsidian bundle**: Addition of the `Vault` class (`SynapseVault`), `Workspace` (`SynapseWorkspace`), `SuggestModal`, `loadMermaid` (with a safe dynamic import), `loadMathJax` and the `getIconIds` function.

- **Obsidian API compatibility layer: Support for Vault, Workspace and App**:
  - **Options and traversal in `SynapseVault`**: Methods `vault.getConfig` (`vimMode`, `theme`, `legacyEditor`, `livePreview`), `vault.getName` and `recurseChildren(folder, callback)` for recursive iteration of file hierarchies.
  - **Extensions in `SynapseWorkspace`**: `containerEl` property, methods `getMostRecentLeaf()` and `iterateAllLeaves(callback)`, event firing with `updateOptions()` and registration of hover preview sources (`registerHoverLinkSource` / `unregisterHoverLinkSource`).
  - **Local persistence and properties in `SynapseApp`**: Methods `loadLocalStorage` and `saveLocalStorage` for plugins that store temporary settings, `app.foldManager` object (`load`, `save`) for code/section folding support and `app.isMobile = false` flag for compatibility with calendar plugins and daily note views.
  - **Metadata cache**: Method `metadataCache.getCachedFiles()` to query all the paths registered in the cache.

- **Obsidian API compatibility layer: Plugin ecosystem, UI and Status Bar**:
  - **Lifecycle in `Plugin`**: Methods `registerObsidianProtocolHandler`, `registerEditorSuggest` and `registerHoverLinkSource` with automatic cleanup in `onunload`.
  - **Centralized registry in `PluginLoader`**: Exposure of the `app.plugins.plugins[pluginId]` map assigned before the initialization of each plugin and cleaned up on unload.
  - **`SynapseInternalPluginsManager` manager (`app.internalPlugins`)**: Support for native plugins (`app.internalPlugins.getPluginById` and `getEnabledPluginById`), required by community plugins such as `Calendar`, `Periodic Notes` and the `obsidian-daily-notes-interface` library.
  - **Native `daily-notes` support**: Registration of the internal `daily-notes` plugin with default settings (`YYYY-MM-DD`, folder and template), asynchronously loading the options from `.obsidian/daily-notes.json` or `.synapse/daily-notes.json`.
  - **Internal plugins and stubs**: Registration of `workspaces`, `file-recovery` (with safe asynchronous handlers) and stubs for `backlink`, `outgoing-link`, `tag-pane`, `page-preview` and `graph`.
  - **Status bar integration**: `SynapseStatusBarManager` directly connects the `.status-bar-item.plugin-status-item` elements created by plugins with the `StatusBar.svelte` visual component at the bottom of the workspace.
  - **UI components and modals**: Base class `SuggestModal<T>` with a results container and a connected `FuzzySuggestModal<T>`; fluent methods `setPlaceholder` and `setInstructions` in `Modal`.

- **Obsidian API compatibility layer: Native DOM extensions and `moment()` library**:
  - **Native DOM extensions in `HTMLElement.prototype`**: Canonical Obsidian methods (`setText`, `getText`, `addClass`, `removeClass`, `addClasses`, `removeClasses`, `toggleClass`, `hasClass`, `setAttr`, `getAttr`) in the global DOM prototype for safe manipulation without runtime exceptions.
  - **Extended date and locale support in `moment()`**: Comparison and boundary methods (`isSame`, `isBefore`, `isAfter`, `startOf`, `endOf`), unit manipulation (`year`, `month`, `date`, `day`, `weekday`, `isoWeekday`, `week`, `set`, `get`), relative formatting (`calendar()`), internationalization methods (`locale`, `localeData`, `moment.locale`, `moment.localeData`, `moment.updateLocale`, `moment.locales`, `moment.weekdays`, `moment.weekdaysShort`, `moment.weekdaysMin`) and the global spec `_bundledLocaleWeekSpec`.

- **In-memory folder indexing (`DashSet`) and fast navigation**:
  - **`NavigationEngine` (Rust backend)**: New concurrent collection `folders: Arc<DashSet<CompactString>>` to register the vault's directories (including empty folders).
  - **Cold hydration (< 1 ms)**: Instant pre-population of ancestor folders at startup from the notes' in-RAM paths without breaking the binary compatibility of the `bincode` cache.
  - **Reactive filesystem watcher**: Live synchronization of folders on creation, deletion and rename events on disk.
  - **Pure domain service (`domain/services/folder_service.rs`)**: Methods `extract_ancestor_folders` and `summarize_folder` to compute the semantic footprint (`notes_count`, `subfolders_count`, `dominant_tags` and `sample_notes`) in memory with no direct I/O.
  - **Tauri command `search_folders_command`**: Fuzzy, paginated folder search returning the semantic footprint.

- **Advanced agentic MCP tools for folders**:
  - **`search_folders`**: Lets LLMs search folders with fuzzy matching and receive their semantic footprint (count, most frequent tags and sample notes) to disambiguate same-named folders in a single call without overloading the context window.
  - **`move_folder_contents`**: Moves notes in bulk between directories with prevention of path traversal and infinite loops, optional filter by extension, a conflict resolution strategy (`skip`, `rename`, `overwrite`) and `dry_run` support.
  - **Pagination in `list_folder_contents`**: Support for `limit` and `offset` with `has_more` and `next_offset` for directories with hundreds of notes.
  - **Path filter in `full_text_search`**: Support for partial matches by folder segment with high recall.

- **Improved UX in `Ctrl+O` (Quick Open)**:
  - **`/` mode (`DIR`)**: Typing `/` filters exclusively folders in the vault with a visual indicator badge.
  - **Smart collapse to Top 3 in normal search**: In general queries, matching folders are limited to the 3 most relevant ones to avoid pushing notes and files down.
  - **Interactive "See X more folders..." element**: Expand/collapse toggle with full mouse click or keyboard (`Enter`) support.
  - **Interactive drill-down and Reveal button**: Pressing Enter on a folder autocompletes the search to `path/folder/`, or a button to reveal it directly in the side explorer.

- **Unified page navigation with relative height retention (`PageDown` / `PageUp`)**:
  - **Shared module `shared/lib/navigation` (`relative-page-navigator.ts`)**: Reusable modular utility that computes the vector $\Delta Y = Y_{\text{target}} - Y_{\text{current}}$ and scrolls the container ensuring that the selected element keeps the same visual row or height on screen (Relative Viewport Retention).
  - **QuickOpen (`Ctrl+O`)**: Adaptive navigation at a fixed 60 FPS that preserves the vertical position of the selected element or safely clamps at the ends.
  - **Full-Text Search (`Ctrl+Shift+F`)**: Full support for `PageDown` and `PageUp` in `FullTextSearch` with relative position retention and focus synchronization.
  - **VaultExplorer (side explorer)**: Support for `PageDown` and `PageUp` with relative navigation for both single selection and range blocks with `Shift`.

### Fixed

- **Performance and smoothness optimization in Excalidraw editing**:
  - **Elimination of typing delay**: Scene serialization and LZ compression on the main thread were suspended while the user is actively editing text (`appState.editingTextElement`), preventing characters from being held in a buffer and drawn with a lag.
  - **Decoupling from Svelte 5's reactive Proxy**: Replacement of `$state<any>(null)` with a plain non-reactive variable for `excalidrawAPI` in `ExcalidrawViewer`, eliminating Proxy interception overhead on Excalidraw's methods and complex internal structures.
  - **Protection against destructive scene overwriting**: In the synchronizing `$effect`, calling `excalidrawAPI.updateScene()` during active text editing is avoided to prevent the momentary blocking and freezing of the WYSIWYG textarea.
  - **Elimination of double compression and serialization in `isTabDirty`**: `WorkspacePage` now checks the dirty state through a direct in-memory comparison (`note.content !== note.savedContent`), dispensing with synchronous calls to `getCurrentSerializedScene()` on every reactive re-render of the tab bar.
  - **Skipping word count in drawings**: `docWordCount` and `docCharCount` are skipped when the active tab is a drawing (`isDrawingTab`), saving costly regular expression runs over JSON and base64.
  - **Lazy SVG generation in Obsidian Excalidraw**: In `ObsidianExcalidrawViewer`, vector generation with `ExcalidrawAutomate.toSVG` now runs exclusively in visual mode (`viewMode === 'visual'`), avoiding heavy computations on every keystroke in code mode.
  - **Suppression of redundant backend reloads on `vault:files-changed`**: If all the files notified by the system watcher come from our own recent saves within the grace window and there are no deleted files, `fetchNotesFromBackend()` and `refreshTree()` are skipped, refreshing only the Git state in a decoupled way.

- **Elimination of the double execution of `onload()` in the plugin lifecycle**:
  - In `PluginLoader`, fix of the redundant invocation of `instance.onload()` after `instance.load()`, ensuring that `onload()` runs only once per active plugin.
  - Support for asynchronous lifecycle methods in `Component.prototype.load()` and `Component.prototype.unload()`, properly awaiting promises returned by `onload()` and `onunload()`.

- **Defensive protection against stack overflow (`Maximum call stack size exceeded`) from monkey-patching**:
  - **`SynapseDataAdapter` (`adapter.read`, `readBinary`, `write`)**: Detection and automatic cutting of recursive cycles in simultaneous reads and writes on the same active path caused by community plugins (e.g. `gpg-crypt`), safely delegating to the base repository without exhausting the call stack.
  - **Internal `file-recovery` plugin**: Per-file re-entry guards (`activeFileRecoveryPaths`) in `onFileChanged` and `forceAdd`, ensuring that circular interceptions of modification events are resolved without infinite recursion.

- **Isolation of community plugins in `.synapse/plugins`**:
  - Restriction of plugin discovery, scanning, loading and uninstallation to operate autonomously in the vault's `.synapse/plugins/` directory, avoiding collisions or interference with the native `.obsidian/plugins/` configuration.

- **Stability and robustness in the Markdown editor (CodeMirror 6)**:
  - **Transparent recovery from extension reconfiguration**: Detection of the `Field is not present in this state` error when dynamically injecting plugin StateFields, recreating the full `EditorState` without interrupting the user and preserving the selection position and the active document.
  - **Debounce on plugin extension updates**: 150 ms debounce in `WorkspacePage` for the `editor-extensions-change` event, grouping the concurrent initialization of multiple community plugins.

## [2026.10.27.04-alpha] - 2026-10-27

### Added

- **Folder creation and visibility in VaultExplorer**:
  - **Direct folder creation**: Quick-access button with a `FolderPlus` icon in the vault explorer header and a "New folder" option in the file tree's context menu.
  - **Support and persistence of empty folders**: It is guaranteed that empty folders are listed and remain visible in the explorer hierarchy without being hidden or discarded in file filtering.
  - **Support for renaming folders**: Seamless inline renaming for folder-type nodes in the tree.

- **Git Push command and keyboard shortcut (`Ctrl+Shift+K`)**:
  - **Command palette (`Ctrl+P`)**: New command "Vault: Push changes to Git (Push)" registered in the workspace command palette.
  - **Configurable global shortcut**: Assigned by default to `Ctrl+Shift+K`, configurable in `config.jsonc` and managed through the application's shortcut store.
  - **Backend and Onion Architecture**:
    - `git_push` port in the `GitPort` trait in `domain/services/git_service.rs`.
    - `git_push` use case in `application/use_cases/git_use_cases.rs`.
    - Infrastructure adapter in `infrastructure/services/git_service.rs` invoking `git push` with safe error handling.
    - Exposure of the Tauri command `git_push_vault` in `infrastructure/tauri/commands/git.rs`.
  - **Visual feedback**: Contextual success and error toast notifications in the user interface when syncing with the remote repository.

- **Creation of D2 diagram files (`.d2`)**:
  - **Option in context menu and command palette**: "New D2 diagram" action added to the file/folder context menu and to the command palette (`cmd-new-d2`).
  - **Default template**: Automatic generation of initial source code (`x -> y: conexión\n`) and direct opening in the D2 diagram viewer.

## [2026.10.27.03-alpha] - 2026-10-27

### Added

- **Context menu and URL copying for the MCP Server in the status bar**:
  - **Accessible context menu with Bits-UI (`ContextMenu.Root`)**: Lets you right-click the MCP indicator in the status bar to quickly access key actions.
  - **Action to copy the MCP URL**: "Copy MCP server URL" option (`/mcp` endpoint with dynamic port) to the clipboard with visual feedback through a toast in the status bar.
  - **Quick action to toggle the server**: Toggle activation/deactivation of the MCP server for the current vault directly from the context menu.
  - **Reactive state synchronization in WorkspacePage**: Calls to `refreshMcpStatus()` on vault opening, on loading recent folders and on initializing workspace tabs.

### Changed

- **Backend dependency updates (Rust / Cargo)**:
  - Update of the full-text search engine `tantivy` to `0.27.0`.
  - Update of the XML parser `quick-xml` to `0.42.0`, adapting `docx_extractor.rs` to the new `&str` typing of element names and text events.
  - Update of `lopdf` to `0.46.0` for PDF extraction and indexing.
  - Pinned the `bincode` dependency at `2.0.1` (with the `serde` feature), guaranteeing stable and compatible binary serialization in the cache.

### Added

- **Search and Exploration by Tags (#tags) in Quick Open (`Ctrl+O`)**:
  - **In-RAM Integration (`DashMap<NoteId, NoteMeta>`)**:
    - Persistence of `tags: Vec<CompactString>` directly in the hot `NoteMeta` structure, automatically extracting tags (both YAML frontmatter and inline `#tag` hashtags) during reconciliation and change monitoring (`watcher`).
    - Ultra-reduced memory consumption (< 1 MB of RAM for 10,000 notes) through inline `CompactString` with no heap allocations for tags of up to 24 bytes.
    - `get_vault_tags()` and `get_notes_by_tag()` now resolve in memory in **< 0.5 ms** (eliminating I/O and synchronous disk reads).
  - **Multithreaded Indexing in `nucleo`**:
    - Injection of tags into the `nucleo` fuzzy engine to allow matches at 60 FPS when searching by tag name with or without the `#` symbol.
    - Full support for hierarchical sub-tags with flexible matching (`linux/cachyos` responds to queries like `#linux` or `#cachyos`).
    - Combined text and tag search (e.g. `kernel #cachyos`).
  - **Frontend User Experience (`QuickOpen.svelte`)**:
    - Tag exploration mode: when typing `#`, it suggests the list of available tags with a dynamic count of associated notes and one-click autocompletion.
    - Interactive tag badges/pills in the search results: they let you refocus the query on a specific tag by clicking.
    - Visual indicator and `TAG` badge in the search box.

- **Frontmatter Display in Reading Mode (Reading View)**:
  - **Interactive Properties Panel (`NoteProperties.svelte`)**:
    - Clean, structured display of YAML metadata at the top of the Markdown document in reading mode, avoiding rendering code blocks or raw triple dashes.
    - Smart detection of property types: interactive tags (`#tag`) with a search action, dates with contextual icons, links and wikilinks (`[[Nota]]`), boolean badges (`Verdadero` / `Falso`) and arrays/lists in compact chips.
    - Copy-to-clipboard button on hover for each property and a collapsible header with a dynamic metadata count.
  - **Isolation and Clean Extraction (`frontmatter-extractor.ts`)**:
    - Atomic separation of YAML metadata and the Markdown body before processing with `marked`, ensuring that the document content does not show frontmatter duplication.
    - Exhaustive unit test suite with Bun/Vitest (`frontmatter-extractor.test.ts`).
  - **User Preferences (`propertiesViewMode`)**:
    - Support in `appSettings` and `settings.svelte.ts` for the modes `'expanded'` (expanded by default), `'collapsed'` (collapsed by default) and `'hidden'` (hidden).

- **MCP Capabilities Optimization (Sub-tags, Whitespace Tolerance and Ergonomics)**:
  - **Hierarchical Sub-tag Matching in `search_notes` and `full_text_search`**:
    - Addition of `matches_tag` and `any_tag_matches` in [`tag_service.rs`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src-tauri/src/domain/services/tag_service.rs), allowing searches by tags like `cachyos` or `linux` to automatically match hierarchical sub-tags like `linux/cachyos` or `os/linux/cachyos`.
    - Unified support for YAML tags and inline hashtags (`#etiqueta`) extracted from the note body.
    - Optional `query` argument in `search_notes`, making filtering exclusively by tags easier.
  - **Clean Snippets and Metrics in `search_notes`**:
    - Return of `snippet` (first 200 clean characters of the body without frontmatter or code delimiters), `description` (from `extra_properties`), `total_lines` and `estimated_tokens`, avoiding unnecessary full reads with `read_note`.
    - In-memory loading of the Markdown content in `FileNoteRepository::walk_directory` to deliver instant results.
  - **Tolerant Surgical Patch and Fuzzy Anchoring in `patch_note`**:
    - Pure function `apply_surgical_patch` with replacement at multiple tolerance levels: exact match, `\r\n` to `\n` normalization (CRLF/LF), and fuzzy line anchoring ignoring trailing whitespace (`.trim_end()`) and indentation (`.trim()`).
    - Automatic disambiguation of multiple occurrences using `start_line`.
    - Enriched error messages that show the closest actual surrounding lines in case of a match failure.
  - **Reading Ergonomics in `read_note`**:
    - Parameter `outline_only: true` (and `outline_only` mode) to obtain the TOC structure and metadata without downloading the full content.
    - Parameter `section` as an alias of `section_heading` with automatic inference of the `section` mode, and a report of available sections if the requested heading is not found.

- **MCP `delete_note` Tool and Standardization of Normative Annotations**:
  - **`delete_note` (New)**: Permanent and safe deletion of notes on disk with strict validation of vault boundaries (Path Traversal prevention) and support for Optimistic Concurrency Control through `expected_hash`.
  - **Normative Standardization of MCP Annotations (`McpToolAnnotations`)**: `readOnlyHint`, `destructiveHint`, `idempotentHint` and `openWorldHint` metadata aligned with the official MCP specification and the de facto standard of `@modelcontextprotocol/server-filesystem`:
    - `openWorldHint: false` across the whole catalog (closed local environment).
    - `destructiveHint: true` on operations that overwrite or delete data (`write_note`, `patch_note`, `update_note_metadata`, `move_notes`, `delete_note`).
    - `append_to_note` classified as the only purely additive write operation (`destructiveHint: false`, `idempotentHint: false`).
    - `idempotentHint: false` for relative, state-dependent or single-use operations (`patch_note`, `move_notes`, `delete_note`, `append_to_note`).

## [2026.10.27.02-alpha] - 2026-10-27

### Added

- **Evolution of the Unified MCP Tool Catalog for Synapse**:
  - **Catalog of 14 Surgically Precise, Multimodal Tools**:
    - **Optimistic Concurrency Control (OCC) and Cryptographic Hashing**: Added `calculate_content_hash` with SHA-256 (`sha256:<hex>`) in `read_note`, `write_note`, `patch_note`, `append_to_note` and `update_note_metadata` to prevent blind overwrites between agents and the editing UI.
    - **Extended `read_note`**: `full`, `chunk` (bounded range with `start_line` and `line_count`), `section` (isolated heading extraction) and `metadata_only` modes. Generates a Table of Contents (TOC) with line numbers, a count of `mermaid`/`d2` blocks, a token estimate (`estimated_tokens`) and an optional line-number prefix (`include_line_numbers`).
    - **Robust `write_note`**: Optional title that preserves existing OKF metadata, an anti-truncation brake (>150 lines and >40% reduction), `expected_hash` validation and a `dry_run` preview that generates a `unified_diff`.
    - **`patch_note` (New)**: Surgical snippet editing with atomic replacement, fuzzy anchoring that tolerates offsets, OCC concurrency control and unified diff generation (`similar`).
    - **`search_in_note` (New)**: Contextual grep inside a note that returns the line number, the match and the surrounding context lines (`context_lines`).
    - **`append_to_note` (New)**: Direct insertion at the end of the document or in specific heading sections (`start_of_section` or `end_of_section`).
    - **`update_note_metadata` (New)**: Atomic mutation of the YAML frontmatter only (add/remove tags without duplicates, update the title, aliases and custom properties) while leaving the body intact.
    - **`read_pdf_document` (New)**: PDF document inspection with `lopdf` (`outline_only` mode, reading by page range with a strict limit of 5 pages to protect the context window, and internal `search`).
    - **`extract_note_diagrams` (New)**: Specialized extraction of Mermaid and D2lang diagrams with start/end lines, or direct reading of `.d2` and `.mmd` files.
    - **Enriched `get_backlinks_and_relations`**: Includes contextual snippets of the line containing the link, detection of orphan/unresolved links (`unresolved_links`) and Jaccard similarity of tags to find the top 5 conceptually related notes (`shared_tags_neighbors`).
    - **`move_notes` with Wikilink Rewriting**: Automatic rewriting of `[[...]]` references in the vault when `update_wikilinks: true`.
    - **MCP Annotations (`McpToolAnnotations`)**: `readOnlyHint`, `destructiveHint` and `idempotentHint` metadata for safe Human-in-the-Loop governance in ACP clients and the UI.

- **Context Menu and Git Status in File Breadcrumbs**:
  - **Visual Git Status Indicators in Breadcrumbs**:
    - Color-coded badges according to the Git status are shown in the path bar (`BreadCrumb.svelte`) next to the active file:
      - Green `M` for staged changes (`index: 'M'`).
      - Green `A` for new staged files (`index: 'A'`).
      - Red `M` for unstaged local modifications in the worktree (`worktree: 'M'`).
      - Amber `?` for untracked files (`worktree: '?'`).
      - Purple `S` for files with stashed changes (`is_stashed: true`).
    - Decoupled normalization logic shared in `@entities/git` (`parseGitStatusInfo`, `NormalizedGitStatus`), respecting FSD and avoiding code duplication.
  - **Extended Context Menu with Full Git Support**:
    - Right-clicking the active file segment in the breadcrumb opens an accessible context menu (`bits-ui ContextMenu`) with key file options and a full suite of Git actions:
      - *Commit...*: Directly opens the Git commit dialog for the current file.
      - *View differences (Diff / Diff staged)*: Support for text diffs and visual (2-pane) diffs for image files, diagrams or PDFs.
      - *Git history (Log)...*: Opens the revision history dialog of the active file.
      - *Git Add*: Stages the active file directly into the Git index/stage with a toast notification.
      - *Git Restore --staged*: Unstages the file (`git restore --staged`).
      - *Git Restore*: Opens a safe confirmation dialog and discards the local modifications in the working tree.
      - *Open in new window* (detaches the file or opens it in an independent window).
      - *Open with the default app* of the operating system.
      - *Show in folder* (reveals the file in the native file explorer).
      - *Rename file* (enables direct inline editing of the breadcrumb).
      - *Copy path* (copies the file's relative path to the clipboard).
    - **Reuse and FSD Architecture**: The [`GitRestoreDialog`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src/shared/ui/git-restore-dialog/) component was promoted to `@shared/ui` so it can be reused by both the vault explorer (`VaultExplorer`) and the breadcrumb (`BreadCrumb`).

- **Per-Vault Persistence and Global Port Registry for the MCP Server**:
  - **Per-Vault Remembered State and Port**:
    - Each vault stores its activation preference (`enabled: true/false`) and its assigned TCP port in `<vault>/.synapse/mcp.json`.
    - When opening or switching vaults (`activate_vault`), Synapse automatically synchronizes the MCP server, starting or stopping the runtime as appropriate for that specific vault without interfering with others.
  - **Global Registry in `~/.config/synapse/mcp-ports.json`**:
    - The mapping of all vaults, assigned ports, active process PID and run state is recorded in `~/.config/synapse/mcp-ports.json` (and `~/.config/synapsers/mcp-ports.json`).
    - Lets multiple concurrent `synapsers` / MCP instances coordinate and discover which ports are actively in use.
    - **Smart Conflict Prevention**: An assignment algorithm that detects dead processes and looks for the next available TCP port (range `27182-28000`) when the preferred port is already taken by another vault or service.
    - Automatic cleanup when the main Synapse window is closed to release the registered port.
    - New Tauri command `get_mcp_port_registry` and updated types in `system.api.ts`.

- **Vim Shortcut Navigation in Reading Mode**:
  - Full support for scrolling through Markdown notes in reading mode using standard Vim / *pager* motion keys:
    - **Vertical scrolling**: <kbd>j</kbd> (down) and <kbd>k</kbd> (up) with continuous, delay-free stepping while the key is held down; <kbd>d</kbd> / <kbd>Ctrl+D</kbd> (half page down) and <kbd>u</kbd> / <kbd>Ctrl+U</kbd> (half page up) with smooth scrolling; <kbd>Ctrl+E</kbd> / <kbd>Ctrl+Y</kbd> (line-by-line scrolling).
    - **Jump to start and end**: <kbd>gg</kbd> (quick double press of `g`) to scroll to the start of the document and <kbd>G</kbd> (<kbd>Shift+G</kbd>) to go straight to the end.
    - **Horizontal scrolling**: <kbd>h</kbd> (left), <kbd>l</kbd> (right), <kbd>0</kbd> (horizontal start) and <kbd>$</kbd> (horizontal end).
    - **Interactive Vim-style search**: <kbd>/</kbd> to open the in-document search bar, <kbd>n</kbd> / <kbd>N</kbd> to move between matches and <kbd>Escape</kbd> to close the search, immediately returning focus to the document so you can keep navigating.
    - **Safe isolation**: keys are not intercepted when the user is typing in a text field, `<input>` or `<textarea>`.

### Fixed

- **Automatic Focus on Markdown Content When Switching Tabs**:
  - Fixed the loss of focus when switching tabs (by clicking a tab, using the <kbd>Ctrl+Tab</kbd> / <kbd>Ctrl+PageDown</kbd> shortcuts or from QuickOpen): focus is now automatically transferred to the main content of the note (`.markdown-reading-wrapper` in reading mode and the CodeMirror `editorView` in edit mode).
  - Lets you use the keyboard arrows (<kbd>ArrowDown</kbd>, <kbd>ArrowUp</kbd>, <kbd>PageDown</kbd>, <kbd>PageUp</kbd>) immediately without having to click the document with the mouse.
  - Precise synchronization with the Svelte 5 lifecycle and `ResizeObserver` to grant focus at the exact moment the tab goes from `display: none` (`.tab-pane.hidden`) to visible, respecting active modal dialogs (`[role="dialog"]`).

## [2026.10.27.01-alpha] - 2026-10-27

### Added

- **Multilanguage Support (i18n) with Svelte 5 Runes**:
  - Native internationalization in `src/shared/lib/i18n/` with no external dependencies:
    - Spanish (`es.ts`) and English (`en.ts`) dictionaries with strict TypeScript typing (`TranslationKey`, `LocaleSchema`) and compile-time key parity checking.
    - Reactive `I18nManager` (`i18n.svelte.ts`) using Svelte 5 `$state` and a `t(...)` translation function that supports dynamic variable interpolation (`{param}`).
    - Automatic detection of the preferred system/browser language and persistence in `localStorage` synchronized with `AppSettings`.
  - Full integration into the global settings model (`appSettings.locale`, `setLocale`, `toggleLocale`).
  - Quick language switch button in the side toolbar (`Ribbon`) with the `Languages` icon, and in the top bar of the welcome screen (`WelcomeScreen`).
  - New command in the command palette (<kbd>Ctrl+P</kbd>): *"Change interface language (Español / English)"* under the *"Settings"* category.
  - Reactive internationalization of the sidebar (`Ribbon`), the welcome screen (`WelcomeScreen`), the relative time formatter and note count plurals.

- **Search in the Read-Only Markdown View (Reading Mode)**:
  - Support for searching text inside Markdown notes in interactive reading mode:
    - Floating search bar available with <kbd>Ctrl+F</kbd> / <kbd>Cmd+F</kbd>.
    - Visual highlighting in the document (`<mark class="reading-search-highlight">`) with smooth automatic scrolling to the selected match (`.reading-search-active`).
    - Real-time match counter (`X of N` or `No matches`).
    - Bidirectional navigation between results using <kbd>Enter</kbd> (next), <kbd>Shift+Enter</kbd> (previous) or interface buttons.
    - Case-sensitive matching toggle with the `Aa` button.
    - Quick close and highlight cleanup using the <kbd>Escape</kbd> key or the close button.
  - Transparent integration in `MarkdownViewer.svelte` and the global shortcut dispatcher of `WorkspacePage.svelte`.
- **Circular Tab Navigation with Keyboard Shortcuts**:
  - Support for quickly switching between open tabs:
    - **Next tab**: <kbd>Ctrl+PageDown</kbd> or <kbd>Ctrl+Tab</kbd>.
    - **Previous tab**: <kbd>Ctrl+PageUp</kbd> or <kbd>Ctrl+Shift+Tab</kbd>.
  - Endless circular behavior (at the end it wraps around to the first tab and vice versa).
  - Pane awareness in split view (*Split View*), cycling within the active pane (`activeFocusedPane`).
  - Robust normalization of key combinations and aliases in `matchesShortcut` (`pageup`, `pgup`, `prior`, `pagedown`, `pgdn`, `next`).
  - Configurable in `shortcuts.jsonc` and registered in the command palette (`Ctrl+P`).
- **Cascade Closing of Detached Windows When the Main Window Is Closed**:
  - When the main Synapse window (`main`) is closed, whether from the window controls or by shortcuts or system events, all detached secondary windows (`detached windows`) are closed automatically.
  - Coordinated implementation at the Tauri backend lifecycle level (`on_window_event` on `Destroyed` of the `main` window) and in the `WorkspacePage` frontend (`onCloseRequested` and window close confirmation).

### Fixed

- **Keyboard Navigation Smoothness, Search Performance and Reactive Loop Fixes**:
  - **Fix for `effect_update_depth_exceeded`**: Removed infinite update cycles in `QuickOpen.svelte` and `WorkspacePage.svelte` by isolating reactive state mutations with Svelte 5's `untrack()` and previous-change guards in the `$effect` blocks (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **`O(1)` Shortcut Precompilation**: In `shortcuts.svelte.ts`, a cache of parsed combinations (`comboCache`) was implemented to avoid running `.split(',')`, `.split('+')` and `.toLowerCase()` on every key press.
  - **Fast-Reject in the Keyboard Dispatcher**: In `WorkspacePage.svelte`, an immediate `O(1)` check was added that ignores keys without modifiers (`Ctrl`/`Meta`/`Alt`/`F12`), avoiding the evaluation of all 19 shortcut checks when navigating with arrows or typing text.
  - **Listener Isolation and Centralization**: Removed the redundant `window.addEventListener('keydown')` listener in `CommandPalette.svelte`, centralizing global shortcuts in `WorkspacePage`.
  - **Rendering Optimization in WebKitGTK**: Replaced the costly full-screen `backdrop-filter: blur(10px)` with semi-transparent solid backgrounds in `QuickOpen.svelte`, `CommandPalette.svelte` and `FullTextSearch.svelte`, removing the `transition` on result rows and adding `contain: layout style paint` for instant row changes (a single frame).
  - **Indexed Search in the QuickOpen Fallback**: Indexed the `vaultItems` in a `Map` to resolve lookups in $O(1)$ instead of running repeated $O(N)$ `.find()` calls.

- **Async Rust Backend Off the GTK Main Thread**:
  - Converted to `pub async fn` and `tauri::async_runtime::spawn_blocking` the blocking Tauri commands that ran on the UI main thread: `get_system_theme` (OS detection via `busctl`/`gsettings`), `search_notes_command` (fuzzy search in Nucleo), `get_vault_git_status` (Git status check on window focus) and the workspace management commands (`workspace.rs`).
  - Derived `Clone` on `GitUseCases` and `GitService`, respecting the Onion Architecture.

- **Cleanup of False Console Errors for Community Plugins**:
  - Silenced the error messages in `TauriVaultRepository.readNote` for optional configuration files (`.synapse/` and `.obsidian/`) that do not exist in newly opened vaults.
  - Fixed the default value of `getEnabledPlugins()` to return an empty list `[]` if the vault has no community plugins configured or installed, avoiding unnecessary load attempts.

- **Performance Optimization and Removal of Freezes ("Micro-Stutters") When Saving and Modifying Files**:
  - **Async Rust Backend**: Replaced the synchronous, blocking `reconcile_sync()` calls with [`spawn_reconciliation()`](file:///home/miuler/src/github.com/Miuler/synapse.rs/src-tauri/src/infrastructure/navigation/engine.rs#L313) in the Tauri commands for saving notes (`save_note_content`), creating folders (`create_vault_folder`), file operations (`delete_vault_item`, `rename_vault_item`, `write_binary_file`, `save_binary_asset`) and Git branch switching (`git_checkout_branch`, `git_create_branch`). This keeps the Tauri command thread from being blocked waiting for full disk traversals (`jwalk`).
  - **Debounce on Frontend Events**: Added a debounce timer (350 ms) to the `vault:files-changed` listener in `WorkspacePage.svelte`, avoiding cascading re-renders, heavy Git recalculations and continuous `fetchNotesFromBackend()` calls during rapid operations or bursts of auto-save.

- **Performance and Freeze Prevention in the Text Diff Viewer (Excalidraw and Large Files)**:
  - Made the automatic line expansion in `DiffViewer.svelte` conditional on a new configurable parameter `diff.max_expand_lines` (default `800` lines) in `config.jsonc`.
  - In files that exceed this threshold (such as Excalidraw drawings or large JSONs), only the modified fragments (*hunks* and surrounding context lines) are rendered, avoiding saturation of the main thread and the needless creation of tens of thousands of reactive DOM nodes.
  - Full expansion or collapse remains available on demand through the toolbar controls.
- **Restored Native Escape Behavior in Excalidraw**:
  - Pressing <kbd>Escape</kbd> fully deselects all selected elements and groups, cancels active drawing tools and returns to the selection tool (`selection`), faithfully replicating the Excalidraw web experience.
  - Immediate preservation of focus on the interactive `.excalidraw` canvas so that subsequent direct single-key shortcuts keep responding without additional clicks.
- **Manual Save Protection and Lock Handling for Locked Files**:
  - Fixed the behavior when pressing <kbd>Ctrl+S</kbd> or clicking the save button in notes or diagrams locked with a padlock: explicit confirmation is now requested through a dedicated dialog instead of silently overwriting on disk.
  - When discarding changes in the locked file dialog, the original content saved on disk (`savedContent`) is restored in memory.
  - The editor's top title field is locked as read-only (`readonly`) when the pane is not in edit mode or the file is locked.
  - Strict normalization of lock paths (`normalizeLockPath`) to avoid discrepancies between relative path prefixes (`./` vs clean paths).
- **Stability and Prevention of Edit Lock-Up in Excalidraw**:
  - Optimization in `ExcalidrawViewer` that dynamically updates the React component's properties (`viewModeEnabled`) to avoid desynchronization of internal tools or canvas lock-ups after pressing <kbd>Esc</kbd>.
  - Prevention of redundant auto-save cycles and protected viewer updates.

## [2026.10.26.01-alpha] - 2026-10-26

### Added

- **Embedded Rendering of Excalidraw Drawings in Markdown with Caching**:
  - Full support for visually rendering Excalidraw drawing files (`.excalidraw`, `.excalidraw.json`, `.excalidraw.md`) as SVG vector graphics directly inside Markdown notes.
  - Full compatibility with wikilink transclusions (`![[dibujo.excalidraw]]`, `![[esquema.excalidraw.md|500]]`), standard image syntax (`![](./diagrama.excalidraw)`) and dedicated code blocks (````excalidraw ...````) with support for `{{#include ...}}` directives.
  - **High-Performance Multi-Level Cache System**:
    - **Level 1 (RAM)**: 0 ms in-memory LRU cache for instant display.
    - **Level 2 (Disk)**: Persistence of pre-rendered SVGs in `<vault>/.synapse/cache/excalidraw/<hash>_<theme>.svg`. Reads the SVG in under 2 ms without invoking React or recomputing the geometry if the content has not changed.
    - **Level 3 (Engine)**: On-demand rendering with `exportToSvg`, adapted to Synapse's active light or dark theme.
  - Transparent integration in the CodeMirror live editor (interactive replacement when the cursor moves away, with an edit button) and in the `MarkdownReadingView` reading view.
- **D2 (`.d2`) Diagram Viewer and Editor with WebAssembly (`@terrastruct/d2`)**:
  - Full support for opening, editing and previewing `.d2` diagram files with high-performance native SVG rendering via WASM.
  - Recognition of the `.d2` extension in the `diagrams` category in the Rust backend (`file_types.rs`), in the configuration (`config.jsonc`) and in the frontend entities (`file-types.svelte.ts`).
  - **Declarative In-File Configuration**: Native support for the `vars: { d2-config: { ... } }` block to define the layout engine (`dagre`, `elk`), theme (`theme-id`), dark theme (`dark-theme-id`), sketch mode (`sketch`) and padding directly in the source code.
  - Automatic detection of Synapse's light or dark theme as a fallback if the file does not specify a `theme-id`.
  - Integration in `WorkspacePage`, the diff comparator (`VisualDiffViewer`), the Git revision viewer (`RevisionViewer`) and the rendering of ````d2` code blocks in Markdown notes (both in reading and live editing).
- **Unified `DiagramSplitViewer` Refactoring (`src/shared/ui/diagram-viewer/`)**:
  - Extraction of common diagram viewing components into a single reusable base component, removing more than 1,300 lines of duplicated code between `MermaidViewer`, `MermanViewer` and `D2Viewer`.
  - Unified SVG canvas with interactive zoom and panning over `viewBox`, a Laser Pointer graphics engine on a 2D canvas with a trail, and floating controls with `bits-ui`.
- **Real-Time Autocompletion and Syntax Linter for D2 and Mermaid (`CodeEditor.svelte`)**:
  - **Contextual Autocompletion (`@codemirror/autocomplete`)**:
    - In **D2**: suggestions for engines (`dagre`, `elk`), a catalog of 20 official themes with descriptive names (`0` Neutral, `3` Flagship, `200` Dark Mauve, `300` Terminal, `303` C4, etc.), geometric shapes (`shape:`), directions (`direction:`), styles (`style.*`) and `d2-config` snippets.
    - In **Mermaid**: suggestions for `%%{init: ...}%%` directives, `config:` frontmatter, the catalog of Mermaid.js and Merman themes, `look` (`classic`, `handDrawn`), `layout` (`dagre`, `elk`), `curve` and diagram types (`flowchart`, `sequenceDiagram`, `stateDiagram-v2`, `erDiagram`, `architecture-beta`, etc.).
  - **Inline Linter and Diagnostics (`@codemirror/lint` + `lintGutter`)**:
    - Red wavy underline at the exact line and column of the syntax error returned by the D2 parsers (AST ranges) and Mermaid (`err.hash.loc`), with gutter markers and informative tooltips.
- **Keyboard Shortcut to Toggle/Enter Edit Mode**:
  - New default keyboard shortcut `Ctrl+E` (`toggle_edit_mode`) to instantly switch between reading mode and edit mode in the active Markdown viewer without clicking with the mouse.
  - Registration of the shortcut in the global configuration and in the shortcut map (`shortcuts.svelte.ts`).
- **Global Persistence of Vim Mode**:
  - The Vim mode state (`vimMode`) is now permanently stored and remembered through `localStorage` and the application settings (`appSettings.vimMode`), staying active when Synapse is reloaded or restarted.
- **Bidirectional Synchronization Between Search Modals**:
  - Pressing `Ctrl+Shift+F` while `QuickOpen` (`Ctrl+O`) is open automatically transfers the entered text to `FullTextSearch` and closes the `QuickOpen` window.
  - Conversely, pressing `Ctrl+O` from `FullTextSearch` instantly moves the query to `QuickOpen` and closes the full-text search dialog.
- **Wikilink Autocompletion (`[[...]]`)**:
  - Real-time popup when typing `[[` in the CodeMirror editor, with contextual suggestions of vault notes.
  - Clean formatting: the dropdown list shows only the file's base name, without directory paths or the `.md` extension.
  - Enlarged visual dimensions of the popup menu (`.cm-tooltip-autocomplete`) to 460px wide and up to 420px tall with smooth scrolling.
- **2D and 3D Knowledge Graph with WebGL Acceleration**:
  - Local and global bidirectional knowledge graph viewer with force-directed simulation (*force-directed*).
  - WebGL 3D mode powered by Three.js with an accessible 2D/3D toggle button in the graph viewer header.
  - Support for ghost nodes (*ghost nodes*) for referenced wikilinks that are yet to be created.
  - Multithreaded parallelization with Rayon in Rust for ultra-fast link analysis and graph physics calculation in massive vaults.
- **Tag Explorer and Relations Inspector**:
  - `TagsExplorer` side view to quickly explore, count and filter notes by their tags (`#etiqueta`).
  - `RelationsInspector` panel to audit backlinks, outgoing links and children/parents of the active note.
- **Local MCP Server (Model Context Protocol)**:
  - Built-in MCP stdio server in Rust (`synapse-mcp`) to connect local AI agents to the vault in a dedicated way from the CLI (`synapsers --mcp`).
  - **In-Process (`in-process`) MCP Server with HTTP Transport**: Support for running the MCP server in the background within the same active Synapse desktop instance (`http://127.0.0.1:27182/mcp`), dynamically binding to the vault open on screen and sharing the Tantivy search index in memory.
  - **Indicator and Switch in `StatusBar`**: An interactive button in the bottom status bar that shows the live state (`MCP ON :27182` / `MCP OFF`), lets you turn the server on or off with one click and exposes information about the active vault and requests served via tooltip.
  - **Commands in the Palette (`Ctrl+P`)**: Quick actions to toggle the local MCP server and copy the connection URL to the clipboard.
  - **State Persistence**: Option remembered in `localStorage` (`appSettings.mcpServerEnabled`) to automatically restore the server as on when the app is reopened.
  - Advanced MCP tools for full-text search, relation/backlink inspection, note reading, folder content listing (`list_folder_contents`), listing of neighboring notes in the same folder (`list_sibling_notes`), batch moving of notes between folders (`move_notes`) and Git status.
- **YAML Frontmatter Support According to OKF**:
  - Parsing, indexing and bidirectional preservation of YAML frontmatter metadata aligned with the Open Knowledge Format (OKF) specifications and AI tools.

### Fixed

- **Keyboard Navigation Smoothness, Search Performance and Reactive Loop Fixes**:
  - **Fix for `effect_update_depth_exceeded`**: Removed infinite update cycles in `QuickOpen.svelte` and `WorkspacePage.svelte` by isolating reactive state mutations with Svelte 5's `untrack()` and previous-change guards in the `$effect` blocks (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **`O(1)` Shortcut Precompilation**: In `shortcuts.svelte.ts`, a cache of parsed combinations (`comboCache`) was implemented to avoid running `.split(',')`, `.split('+')` and `.toLowerCase()` on every key press.
  - **Fast-Reject in the Keyboard Dispatcher**: In `WorkspacePage.svelte`, an immediate `O(1)` check was added that ignores keys without modifiers (`Ctrl`/`Meta`/`Alt`/`F12`), avoiding the evaluation of all 19 shortcut checks when navigating with arrows or typing text.
  - **Listener Isolation and Centralization**: Removed the redundant `window.addEventListener('keydown')` listener in `CommandPalette.svelte`, centralizing global shortcuts in `WorkspacePage`.
  - **Rendering Optimization in WebKitGTK**: Replaced the costly full-screen `backdrop-filter: blur(10px)` with semi-transparent solid backgrounds in `QuickOpen.svelte`, `CommandPalette.svelte` and `FullTextSearch.svelte`, removing the `transition` on result rows and adding `contain: layout style paint` for instant row changes (a single frame).
  - **Indexed Search in the QuickOpen Fallback**: Indexed the `vaultItems` in a `Map` to resolve lookups in $O(1)$ instead of running repeated $O(N)$ `.find()` calls.

- **Async Rust Backend Off the GTK Main Thread**:
  - Converted to `pub async fn` and `tauri::async_runtime::spawn_blocking` the blocking Tauri commands that ran on the UI main thread: `get_system_theme` (OS detection via `busctl`/`gsettings`), `search_notes_command` (fuzzy search in Nucleo), `get_vault_git_status` (Git status check on window focus) and the workspace management commands (`workspace.rs`).
  - Derived `Clone` on `GitUseCases` and `GitService`, respecting the Onion Architecture.

- **Cleanup of False Console Errors for Community Plugins**:
  - Silenced the error messages in `TauriVaultRepository.readNote` for optional configuration files (`.synapse/` and `.obsidian/`) that do not exist in newly opened vaults.
  - Fixed the default value of `getEnabledPlugins()` to return an empty list `[]` if the vault has no community plugins configured or installed, avoiding unnecessary load attempts.

- **Prevention of Redundant Reload and Edit Lock-Up in Excalidraw When Pressing Esc**:
  - Fixed a bug in `ExcalidrawViewer` where, when pressing <kbd>Esc</kbd> or on a deselection with loss of focus (`focusout`), serializing and saving the file triggered a reactive external update that overwrote the Excalidraw scene (`updateScene`). This caused the canvas's interactive pointer to be lost and required switching to reading mode and back to edit mode to reactivate drawing.
  - Implementation of local change discrimination (`isLocalChange`) and semantic comparison with a scene version hash (`hasSceneDiverged` using `getSceneVersion`) to prevent rewrites of the live scene generated by the viewer itself.
  - Automatic, guaranteed restoration of the selection tool (`setActiveTool({ type: 'selection' })`) after capturing the `Escape` event.
- **Automatic Focus on the Editor When Reopening or Switching Notes**:
  - When opening or reopening a note that was left in edit mode, or switching between editable tabs, the CodeMirror editor automatically receives input focus (`focus()`), letting you type or scroll with the keys immediately without a prior manual click.
- **Accuracy and Ranking in Fuzzy Search (`search_with_stats`)**:
  - Fixed the ranking algorithm in the Nucleo navigation engine: strict priorities and bonuses are given to exact matches and full substrings in file names and titles, eliminating irrelevant spurious results when searching for common lowercase words (e.g. `todo`).
- **Instant QuickOpen Response (`Ctrl+O`)**:
  - Removed redundant listeners and set `forceMount={true}` on the `Command.Item` elements to prevent the list from going blank.
  - In-memory return in 0 ms if the backend search has not completed its first response.
- **Isolation of Synthetic Graph Tabs**:
  - Fixed path association to prevent synthetic URIs such as `synapse://graph` from being mistakenly identified as empty Markdown notes.
  - Preservation of the previously active note (`lastActiveDocumentPath`) to correctly center the local graph even on isolated notes with no links.
- **Visual Alignment in Full-Text Search**:
  - Fixed the position and alignment of the magnifying glass icon inside `FullTextSearch` and extracted the reusable `SearchBox` component.
- **Deadlock Prevention When Opening or Switching Vaults**:
  - Fixed a thread lock (*mutex deadlock*) in `activate_vault` where the lock on `active_vault_path` and the synchronous notification of the `"mcp:status-changed"` event through `app_handle` competed for resources in Tauri, causing the window to freeze when selecting a vault. MCP event emission is now dispatched decoupled in the background after the locks are released.

### Changed

- **Backend and Frontend Architecture Refactoring**:
  - Modular decomposition of Tauri commands into `session`, `file_ops`, `assets`, `search`, `git` and `workspace`, strictly respecting the Onion Architecture.
  - Purification of the `link_resolution` domain service by decoupling it from external layers.
  - Reorganization of the frontend under canonical Feature-Sliced Design (FSD) principles, extracting modular dialogs (`VaultDeleteDialog`, `VaultRestoreDialog`, `GitManagementDialogs`), canonical APIs and composable models with Svelte 5 Runes.

## [2026.10.24.02-alpha] - 2026-10-24

### Fixed

- **Keyboard Navigation Smoothness, Search Performance and Reactive Loop Fixes**:
  - **Fix for `effect_update_depth_exceeded`**: Removed infinite update cycles in `QuickOpen.svelte` and `WorkspacePage.svelte` by isolating reactive state mutations with Svelte 5's `untrack()` and previous-change guards in the `$effect` blocks (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **`O(1)` Shortcut Precompilation**: In `shortcuts.svelte.ts`, a cache of parsed combinations (`comboCache`) was implemented to avoid running `.split(',')`, `.split('+')` and `.toLowerCase()` on every key press.
  - **Fast-Reject in the Keyboard Dispatcher**: In `WorkspacePage.svelte`, an immediate `O(1)` check was added that ignores keys without modifiers (`Ctrl`/`Meta`/`Alt`/`F12`), avoiding the evaluation of all 19 shortcut checks when navigating with arrows or typing text.
  - **Listener Isolation and Centralization**: Removed the redundant `window.addEventListener('keydown')` listener in `CommandPalette.svelte`, centralizing global shortcuts in `WorkspacePage`.
  - **Rendering Optimization in WebKitGTK**: Replaced the costly full-screen `backdrop-filter: blur(10px)` with semi-transparent solid backgrounds in `QuickOpen.svelte`, `CommandPalette.svelte` and `FullTextSearch.svelte`, removing the `transition` on result rows and adding `contain: layout style paint` for instant row changes (a single frame).
  - **Indexed Search in the QuickOpen Fallback**: Indexed the `vaultItems` in a `Map` to resolve lookups in $O(1)$ instead of running repeated $O(N)$ `.find()` calls.

- **Async Rust Backend Off the GTK Main Thread**:
  - Converted to `pub async fn` and `tauri::async_runtime::spawn_blocking` the blocking Tauri commands that ran on the UI main thread: `get_system_theme` (OS detection via `busctl`/`gsettings`), `search_notes_command` (fuzzy search in Nucleo), `get_vault_git_status` (Git status check on window focus) and the workspace management commands (`workspace.rs`).
  - Derived `Clone` on `GitUseCases` and `GitService`, respecting the Onion Architecture.

- **Cleanup of False Console Errors for Community Plugins**:
  - Silenced the error messages in `TauriVaultRepository.readNote` for optional configuration files (`.synapse/` and `.obsidian/`) that do not exist in newly opened vaults.
  - Fixed the default value of `getEnabledPlugins()` to return an empty list `[]` if the vault has no community plugins configured or installed, avoiding unnecessary load attempts.

- **Auto-save on focus loss and edit mode persistence in Excalidraw and Obsidian Excalidraw**:
  - Implemented instant automatic saving on focus loss (`onfocusout`, `onblur` and `<svelte:window onblur>`) in both the interactive Excalidraw editor and the Obsidian Excalidraw viewer/editor.
  - Fixed the bug where, when saving or blurring the tab, the active edit mode (`isEditing`) reverted to read-only (`reading`), forcing the user to press the reading/edit button again.
  - Reactive synchronization of the `viewMode` of notes open in memory with the pane's active edit state, avoiding mismatches with the file system watcher (`vault:files-changed`).
  - Automatic safeguarding of unpersisted content when components are unmounted (`onDestroy`) and when switching between the canvas view and the source code in Obsidian Excalidraw.

## [2026.10.24.01-alpha] - 2026-10-24

### Added

- **File Locking Against Accidental Modifications (*File Lock*)**:
  - Option to lock and unlock files from the `VaultExplorer` context menu.
  - Padlock icons visible in the file tree, in the tabs (`EditorHeader`) and in the top bar.
  - Strict inhibition of auto-save on locked files, with persistence in `VaultUiState.locked_files`.
  - Accessible confirmation dialog when trying to close tabs or the application with pending changes in locked files, offering options to abort, discard changes or force the save.
- **Reading Mode by Default for Excalidraw Files**:
  - Both native Excalidraw drawings and Obsidian Excalidraw drawings (`.excalidraw`, `.excalidraw.md`) now open in reading mode by default, avoiding accidental in-memory mutations or triggers.
- **System Actions in Context Menus**:
  - **"Open with the default app"**: Lets you open any file with the application configured by default in the operating system (Word, Excel, etc.) from both `VaultExplorer` and the tabs.
  - **"Show in folder"**: Opens the system's native file manager and highlights the selected file or folder.
  - Cross-platform implementation in Rust following the standard FreeDesktop D-Bus specification (`org.freedesktop.FileManager1.ShowItems` with `busctl`/`dbus-send` and fallbacks on Linux, `explorer /select` on Windows and `open -R` on macOS).
- **Local Read and Write Control in the Diff Viewer**:
  - In `VisualDiffViewer`, the local working pane now starts in reading mode by default with a switch to toggle between reading and writing mode exclusively for the local file on disk, while the Git revision pane remains strictly read-only.
- **Support for Embedded Images and Files in Excalidraw Diagrams**:
  - Bidirectional recognition and parsing of the `## Embedded Files` section in Obsidian Excalidraw Markdown files (`.excalidraw.md`).
  - Automatic resolution of relative paths and wikilinks (`fileId: [[ruta/a/imagen.png]]`) to data URLs (`dataURL`) read from the active vault.
  - Dynamic injection into the canvas file registry both in the native Excalidraw viewer (`api.addFiles`) and in the SVG vector generator of `ObsidianExcalidrawViewer` (`ExcalidrawAutomate.toSVG`), allowing embedded images and graphics to display correctly.
  - Full preservation of the `## Embedded Files` section when saving and serializing files.

### Fixed

- **Keyboard Navigation Smoothness, Search Performance and Reactive Loop Fixes**:
  - **Fix for `effect_update_depth_exceeded`**: Removed infinite update cycles in `QuickOpen.svelte` and `WorkspacePage.svelte` by isolating reactive state mutations with Svelte 5's `untrack()` and previous-change guards in the `$effect` blocks (`searchQuery`, `isOpen`, `activeTabPath`, `viewMode`).
  - **`O(1)` Shortcut Precompilation**: In `shortcuts.svelte.ts`, a cache of parsed combinations (`comboCache`) was implemented to avoid running `.split(',')`, `.split('+')` and `.toLowerCase()` on every key press.
  - **Fast-Reject in the Keyboard Dispatcher**: In `WorkspacePage.svelte`, an immediate `O(1)` check was added that ignores keys without modifiers (`Ctrl`/`Meta`/`Alt`/`F12`), avoiding the evaluation of all 19 shortcut checks when navigating with arrows or typing text.
  - **Listener Isolation and Centralization**: Removed the redundant `window.addEventListener('keydown')` listener in `CommandPalette.svelte`, centralizing global shortcuts in `WorkspacePage`.
  - **Rendering Optimization in WebKitGTK**: Replaced the costly full-screen `backdrop-filter: blur(10px)` with semi-transparent solid backgrounds in `QuickOpen.svelte`, `CommandPalette.svelte` and `FullTextSearch.svelte`, removing the `transition` on result rows and adding `contain: layout style paint` for instant row changes (a single frame).
  - **Indexed Search in the QuickOpen Fallback**: Indexed the `vaultItems` in a `Map` to resolve lookups in $O(1)$ instead of running repeated $O(N)$ `.find()` calls.

- **Async Rust Backend Off the GTK Main Thread**:
  - Converted to `pub async fn` and `tauri::async_runtime::spawn_blocking` the blocking Tauri commands that ran on the UI main thread: `get_system_theme` (OS detection via `busctl`/`gsettings`), `search_notes_command` (fuzzy search in Nucleo), `get_vault_git_status` (Git status check on window focus) and the workspace management commands (`workspace.rs`).
  - Derived `Clone` on `GitUseCases` and `GitService`, respecting the Onion Architecture.

- **Cleanup of False Console Errors for Community Plugins**:
  - Silenced the error messages in `TauriVaultRepository.readNote` for optional configuration files (`.synapse/` and `.obsidian/`) that do not exist in newly opened vaults.
  - Fixed the default value of `getEnabledPlugins()` to return an empty list `[]` if the vault has no community plugins configured or installed, avoiding unnecessary load attempts.

- **Freeze Prevention and Memory Optimization in Obsidian Excalidraw**:
  - Removed the CSS `drop-shadow` filter on the SVG container of the Obsidian Excalidraw viewer, preventing WebKitGTK from allocating massive multi-gigabyte rasterized buffers (up to 4 GB) for large vector diagrams.
  - Configured the generated SVG with `width="100%" height="100%"` tied to its `viewBox`, allowing GPU-scaled vector rendering without oversized fixed limits.
  - Implemented a 300 ms debounce on the native Excalidraw editor's `onChange` event to avoid running LZ compression of thousands of elements on every micro-movement of the pointer.
  - Protected loaded images from persisting data URLs or assets in the JSON files block in Markdown documents, respecting the standard `## Embedded Files` format.
- **Opening Excalidraw Diagrams in Reading Mode by Default**:
  - Precise detection of drawing files through their YAML frontmatter metadata (`--- \n excalidraw-plugin: parsed \n ---`) regardless of the name or `.md` extension.
  - Diagrams open by default in reading mode (`readOnly` and button in "Reading" mode) to avoid accidental modifications and unnecessary resource consumption.

- **Edit Mode Persistence During Auto-save in Excalidraw**:
  - Fixed a bug where, when automatically saving a drawing open in Excalidraw, the file reload and the internal scene update reverted the viewer to reading mode or left the active tool in hand mode (`hand`), preventing further editing without toggling the header button.
- **Strict Protection Against Saving Locked Files**:
  - Fixed a bug where `persistVaultItemToRust` persisted locked files when closing the window, unmounting components or firing automatic viewer callbacks. An explicit `force = true` is now required (which can only be triggered via the confirmation dialog, the `Ctrl+S` shortcut or the manual Save button).

## [2026.10.23.04-alpha] - 2026-10-23

### Added

- **Obsidian Community Plugins** (ONLY THE OBSIDIAN EXCALIDRAW PLUGIN WAS VALIDATED):
  - Built-in explorer and installer for community plugins from the official Obsidian catalog.
  - Support for installing, enabling, disabling, uninstalling and running in a secure environment.
  - Two-level cache (RAM and disk) with TTL for instant catalog browsing without saturating the network.
  - Automatic detection of installed plugins by scanning the vault directly on disk.
  - Commands and interface support to check for updates and update plugins automatically.
- **Full-Text Search in PDF and Word (DOCX) Documents**:
  - Automatic text extraction and indexing for `.pdf` and `.docx` files in the Tantivy engine in Rust.
  - Detailed configuration in `config.jsonc` (`indexing`): size limits (`max_pdf_file_size`, `max_docx_file_size`), page limit (`max_pdf_pages`), character limit (`max_extracted_chars`) and filtering of corrupt words (`max_word_length`).
- **Smooth Navigation and Expansion from the Breadcrumb**:
  - Clicking a directory in the `BreadCrumb` of an active note automatically expands and shows `VaultExplorer` if the sidebar was collapsed or hidden.
  - The folder selected in the breadcrumb immediately unfolds its subdirectories and files in the explorer tree.
- **Window Drag Regions on Windows and Desktop Systems**:
  - Enabled native Tauri drag regions (`data-tauri-drag-region`) in the empty areas of the tab header (`EditorHeader`) and in the explorer header (`VaultExplorer`).
  - Dedicated 24px drag spacer next to the window controls to guarantee a comfortable grab point even with dozens of open tabs.
  - Support for maximizing and restoring the window by double-clicking the free areas of the header without interfering with the tabs or the interactive buttons.

## [2026.10.23.03-alpha] - 2026-10-23

### Added

- Explorer, installer and manager for Obsidian community plugins with performance and cache optimizations.

## [0.1.0-alpha] - 2026-10-23

### First Alpha Release

**Synapse.rs** is a hyper-fast, private, *offline-first* app for managing notes, knowledge and diagrams, built with **Tauri v2**, **Rust** and **Svelte 5**. Designed to be lightweight, extensible and compatible with the Markdown and Obsidian ecosystem.

### Added

- **Extreme Speed in Rust**: Instant indexing and navigation with in-memory fuzzy search (*Nucleo fuzzy matching*).
- **Markdown and Diagram Editor**: Native support for Markdown, Mermaid diagrams, Merman and Excalidraw drawings (`.excalidraw.md`).
- **Integrated Git Version Control**: Change history, visual per-file diff, branching and stashing directly from the app.
- **Standalone File Mode (*Standalone*)**: Open and edit individual files (`.md`, `.mermaid`, `.excalidraw`) directly without needing to create a workspace.
- **Format Support**: Preview of images, PDF documents and quick access to external files (Word, Excel) with your preferred system viewer.
- **Dynamic Theme**: Dark and Light modes and automatic synchronization with the operating system.
- **100% Private and Local**: Your notes never leave your machine; your files are simply folders on your disk.

### Portable downloads (No installer)

No heavy dependencies need to be installed; just download and run:

| Platform | File | Instructions |
| :--- | :--- | :--- |
| **Linux (x64)** | `synapsers` | Grant execute permission: `chmod +x synapsers` and run `./synapsers` |
| **Windows (x64)** | `synapsers.exe` | Double-click to run directly |

> [!NOTE]
> This is an **Alpha** version. If you find any unexpected behavior or have suggestions, you are welcome to open an issue in the repository.

[1.0.0-beta.9]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.9
[1.0.0-beta.8]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.8
[1.0.0-beta.7]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.7
[1.0.0-beta.6]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.6
[1.0.0-beta.5]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.5
[1.0.0-beta.4]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.4
[1.0.0-beta.3]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.3
[1.0.0-beta.2]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.2
[1.0.0-beta.1]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-beta.1
[1.0.0-alpha.5]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.5
[1.0.0-alpha.4]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.4
[1.0.0-alpha.3]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.3
[1.0.0-alpha.2]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.2
[1.0.0-alpha.1]: https://github.com/Miuler/synapse.rs/releases/tag/1.0.0-alpha.1
[2026.10.27.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.04-alpha
[2026.10.27.03-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.03-alpha
[2026.10.27.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.02-alpha
[2026.10.27.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.27.01-alpha
[2026.10.26.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.26.01-alpha
[2026.10.24.02-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.02-alpha
[2026.10.24.01-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.24.01-alpha
[2026.10.23.04-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.04-alpha
[2026.10.23.03-alpha]: https://github.com/Miuler/synapse.rs/tree/2026.10.23.03-alpha
[0.1.0-alpha]: https://github.com/Miuler/synapse.rs/releases/tag/2026.10.23.02-alpha
