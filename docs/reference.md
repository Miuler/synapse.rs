# Synapse.rs interface reference

The inputs and outputs of the application: command line, files, configuration, and the local MCP server.
For network connections and their privacy implications, see the
[README](../README.md#-qué-sale-de-tu-equipo).

## Command line

```
synapsers [OPTIONS] [path]
```

| Argument | Meaning |
|---|---|
| `path` | A folder (opened as a vault) or a file (opened on its own) |
| `-v`, `--vault <folder>` | Open a folder as a vault |
| `-f`, `--file <file>` | Open a single file, without a vault |
| `--mcp` | Do not open a window: serve MCP over stdio for the vault given with `--vault`, or the current folder |
| `--mcp-proxy <url>` | Forward MCP over stdio to a running instance (used by the integrated agents) |
| `-h`, `--help` | Show help |
| `-V`, `--version` | Show the version |

With `--mcp` and `--mcp-proxy`, standard input and output carry JSON-RPC messages, one per line; diagnostics go to
standard error.

| Environment variable | Effect |
|---|---|
| `SYNAPSE_CONFIG_DIR` | Use this folder instead of `~/.config/synapsers` |
| `SYNAPSE_DMABUF=0` | Linux: disable the fast WebKitGTK renderer if the window is blank or flickers |
| `SYNAPSE_OKF_INDEX=0` | Do not generate the OKF `index.md` files |
| `SYNAPSE_ACP_REGISTRY_URL` | Use another ACP agent registry |
| `AZURE_DEVOPS_PAT`, `ADO_PAT` | Azure DevOps token for task boards, if none is set in Synapse |
| `SYNAPSE_OP_PATH` | 1Password CLI to use instead of `op` from the `PATH` |

## Files

The authoritative list of extensions is [`config.jsonc`](../config.jsonc).

| Kind | Examples | Access |
|---|---|---|
| Markdown notes | `.md`, `.markdown` | Read and edit |
| Diagrams | `.mmd`, `.mermaid`, `.d2`, `.puml`, `.plantuml` | Read and edit |
| Excalidraw drawings | `.excalidraw`, `.excalidraw.md` | Read and edit |
| draw.io diagrams | `.drawio` | Read and edit, with the embedded draw.io editor |
| Code and text | `.yaml`, `.json`, `.sql`, … | Read and edit as text |
| Images | `.png`, `.jpg`, `.svg`, `.webp`, … | Read |
| PDF, Word and spreadsheets | `.pdf`, `.docx`, `.xlsx`, … | Read |

The interface can only read and write inside the open vault and the files the user chooses. A file opened on its own
may also show the images in its folder and subfolders.

Synapse writes to the vault:

- the files the user edits, creates, moves or deletes;
- `.synapse/`: indexes, interface state, plugins, MCP settings and agent conversations;
- `index.md` files with the OKF index, in folders that qualify (can be disabled).

## Configuration

Stored in `~/.config/synapsers/`. Files ending in `.jsonc` accept comments.

| File | Contents |
|---|---|
| `config.jsonc` | Global settings, such as `git.path`, `appearance.theme_dirs` and `issues.connections` |
| `shortcuts.jsonc` | Keyboard shortcuts |
| `themes/*.css` | Custom themes ([guide, in Spanish](temas.md)) |
| `workspaces.json` | Recent and pinned vaults |
| `agents.json` | Custom ACP agents |
| `mcp-ports.json` | MCP port assigned to each vault |
| `wallpapers.json` | Wallpapers chosen by the user |

### Issue provider connections

Task boards sync through connections: any number of them, also several of the same provider. Each one has its own
server, and its token is only sent there (HTTPS, or HTTP to this computer); changing the server discards the token.

```jsonc
{
  "issues": {
    "connections": [
      { "id": "work", "provider": "azure_devops", "name": "Work",
        "server": "https://dev.azure.com/organization", "secret": "op://vault/item/field" },
      { "id": "local", "provider": "azure_devops", "name": "Own server",
        "server": "https://tfs.example.com/tfs" } // token in the keyring, entry `issues-local`
    ]
  }
}
```

`secret` only holds a [1Password secret reference](https://developer.1password.com/docs/cli/secret-references/), never
the token itself; a pasted token is saved in the system keyring (service `synapsers`, entry `issues-<id>`). A board uses
the connection whose server contains its project URL.

Connections are managed in Settings → Connections. «Test» asks the server which account the token signs in with
(`_apis/connectionData`).

With no connections, boards use Azure DevOps (`dev.azure.com` and `*.visualstudio.com` only) with the single token
set in Settings → Connections: a pasted token (keyring) or an `op://` reference in `azure_devops.token`, else
`AZURE_DEVOPS_PAT` / `ADO_PAT`. Synapse reads references with `op read` when a board is synced.

### Which tasks a board brings

Set per board, from the settings button of the board; when a board is created and there is something to sync with,
Synapse asks right away (cancelling keeps the board local). A board takes a connection (or the one whose server covers
the project), a project URL and one of:

- **Mine** (default): open issues assigned to, changed or created by you.
- **By criteria**: type, state, assignee (`@me`, people already seen or typed), iteration (also the current one,
  resolved by dates when pulling), area and tag. Any value of a criterion is enough and every criterion has to match;
  with no state chosen, only open issues.
- **Own query** (WIQL for Azure DevOps).

Criteria are saved in the board together with the query compiled from them; editing the query by hand makes it win.
Linked tasks the query no longer returns stay on the board, marked as outside the filter. Changing the project or the
connection of a board with linked tasks unlinks them, after confirmation. A remote state goes to the column that lists
it in `states` of the board, else to the column of the category the provider declares, else by its usual name.

### Folder boards (`format: 2`)

A board marked `format: 2` keeps one file per task in the folder next to it. `Name.board.md` only holds the board:
`default_view`, `columns`, `column_widths`, `source`, `fields` and the order of the tasks under `## Order` (`- <id>`).
`fields: { card: [key, type, assignee], table: [key, type, assignee] }` chooses what Kanban cards and the table show;
without it cards show the three and the table those that some task has. Each `Name.board/<id>.md` holds a task:

```markdown
# Title

- status: doing
- type: Bug
- assignee: Ana
- key: AB#123
- start: 2026-11-20
- duration: 2d
- depends: -
- progress: 0%

## Description

> Quoted, so nothing in it looks like part of the format.

## Activity

- `2026-11-26T10:00:00` comment · Ana · id:987
  Text of the comment.
```

Unknown keys and sections are kept. The folder can be read but only the board changes it; changes made by other
applications are read again. Existing boards are not converted yet, and new ones are still created as a table.

## MCP server

When enabled for a vault, Synapse serves the [Model Context Protocol](https://modelcontextprotocol.io) (version
`2024-11-05`) as JSON-RPC 2.0, at `http://127.0.0.1:<port>/mcp` with `POST`. The port is 27182 if it is free, or the next
free one up to 28000. The server listens on the loopback interface only and **does not require authentication**.

Methods: `initialize`, `ping`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, `prompts/list` and
`prompts/get`. Resources are the notes of the vault, as `synapse://vault/<path>`. The full input schema of every tool
is returned by `tools/list`.

| Tool | Purpose |
|---|---|
| `search_notes` | Find notes by name, path or title |
| `full_text_search` | Search the contents of the indexed documents: alternatives (`a\|b`, `OR`), matching lines with their location — line, PDF page, Word section and paragraph, spreadsheet cell, draw.io page, model element — (`lines`) and literal texts (`exact`) |
| `search_in_note` | Find one or several texts (`a\|b`) inside one or more notes, with counts |
| `search_folders` | Find folders by name or path |
| `list_folder_contents` (alias `list_vault_tree`) | List the contents of a folder |
| `list_sibling_notes` | List the notes in the same folder |
| `read_note` | Read a note, a section, or its outline |
| `write_note` | Create or replace a note, checking its hash |
| `patch_note` | Replace a fragment of a note |
| `append_to_note` | Add content to a note |
| `update_note_metadata` | Update the frontmatter of a note |
| `move_notes`, `move_folder_contents` | Move notes |
| `delete_note` | Permanently delete a note |
| `get_backlinks_and_relations` | Links to and from a note |
| `get_vault_git_status` | Git status of the vault |
| `git_file_history` | Commits that changed a file |
| `git_diff_revisions` | Changes in a file between two commits |
| `git_read_revision` | Contents of a file in a commit |
| `read_pdf_document` | Read or search a PDF |
| `render_diagram` | A diagram or image of the vault, as an image |
| `extract_note_diagrams` | Extract the diagrams of a note |
| `inspect_archimate_element` | Query an ArchiMate model |
| `list_vaults` | Other vaults that can be read, with the user's permission |
| `list_conversations`, `send_to_conversation`, `read_replies` | Talk to other agent conversations |

Prompts: `resumir_nota` (summarize a note) and `vincular_conceptos` (suggest links).

## Releases

Each release publishes signed packages for Linux, Windows and macOS, `latest.json` for the updater,
`SHA256SUMS.txt` and CycloneDX SBOMs. [SECURITY.md](../SECURITY.md) explains how to verify them.
