# notionsmith

One-way sync daemon: PDF notes exported from any note-taking app -> pages in a single
Notion database, tagged by a `Course` select property.

## Background

Most note apps already have a native, unpaid "export as PDF" feature per note, so
this project consumes those already-rendered PDFs instead of dealing with any
particular app's own file format at all. It started around
[Notein](https://play.google.com/store/apps/details?id=com.orion.notein.global)
specifically, but nothing in the code is Notein-specific: any app that can export a
note to PDF and let the file be named works the same way.

The workflow: export a note as PDF from whatever app you take notes in, name the file
`<PREFIX>_<anything>.pdf` (e.g. `MATHE1_Test1.pdf`), let Syncthing land it in a watched
folder on this machine. This daemon picks it up from there, uploads it to the one
configured Notion database, and sets `Course` to whatever that prefix maps to.

Course names are select options on a single Notion database, not separate databases.
A Relation to a second "Module" database was considered and dropped: a Select the
sync code writes directly (no page-id lookup needed) was simpler and sufficient.
`NOTEIN_COURSE_<PREFIX>` maps a filename prefix to the exact select option string;
`cargo run -- configure` manages that mapping interactively instead of hand-editing
`.env` (see ROADMAP.md).

`configure`'s suggested prefixes can collide (two different course names abbreviating
to the same string) and would otherwise silently overwrite one course's env var with
another's; `disambiguate_prefixes` resolves this before the TUI opens, and saving is
refused outright if a manual edit reintroduces a collision. It also keeps a
prefix -> course reference both as a page inside the Notion database itself (not
just a local file) since Notion, unlike any particular sync tool, is the one thing
every user of this project has.

## Language

All code, comments, doc comments, commit messages, `println!` output, and
documentation (README, `.env.example`) are in **English**. German only in direct
conversation with the maintainer, never in anything committed.

Never use the em dash (`—`) anywhere in this repo, in prose or in code. Use a comma,
colon, period, or parentheses instead. Same goes for chat replies to the maintainer.

## Module layout

- `src/main.rs`: entry point only, env loading, client setup, the outer poll loop. No
  business logic here.
- `src/notein.rs`: everything about the local watch folder, scanning for PDFs and
  parsing the `<PREFIX>_` filename convention. Nothing here knows about Notion.
- `src/notion.rs` + `src/notion/model.rs`: all Notion API interaction (querying
  existing page titles, the file upload, page creation, the reference page). Nothing
  here knows about the local filesystem.
- `src/sync.rs`: the diffing logic and `run_sync_cycle`, the only place that imports
  both `notein` and `notion`.
- `src/history.rs`: persistent local sync history (`history.json`) tracking title, file
  size, and modified timestamp to prevent re-upload loops when pages are deleted from Notion.
- `src/configure.rs`: the interactive `configure` TUI (ratatui/crossterm) and its
  pure prefix-suggestion logic. Imports `notion` for fetching Course options and
  writing the reference page, but owns all of the `.env`-file reading/writing itself.

When adding a new external call, put it in the module for that system, not in
`sync.rs` or `main.rs`.

## Comments

Only comment the **why**, never the what. If you'd delete a comment and nothing about
the code becomes less clear, don't add it in the first place.

## Before committing

Run `cargo build`, `cargo test`, and `cargo clippy --all-targets`. Zero warnings, not
just zero errors.
