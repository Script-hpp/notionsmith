# notionsmith

One-way sync daemon: PDF notes exported from the Notein app -> pages in a single
Notion database, tagged by a `Kurs` (course) select property.

## Background

Notein (com.orion.notein.global) stores notes as `.in` files, which are plain ZIP
archives containing a Room/SQLite database plus JSON metadata. The handwriting itself
is stored as a proprietary protobuf blob (`ink_stroke_blob`, tagged
`notein-fountain-v2`) with no public spec. Reverse-engineering that format to render
the strokes ourselves was considered and dropped: Notein already has a native,
unpaid "export as PDF" feature per note. This project consumes those already-rendered
PDFs instead of touching the `.in` format at all.

The workflow: export a note as PDF from Notein by hand, name the file
`<PREFIX>_<anything>.pdf` (e.g. `MATHE1_Test1.pdf`), let Syncthing land it in a watched
folder on this machine. This daemon picks it up from there, uploads it to the one
configured Notion database, and sets `Kurs` to whatever that prefix maps to.

Course names are select options on a single Notion database, not separate databases.
A Relation to a second "Module" database was considered and dropped: a Select the
sync code writes directly (no page-id lookup needed) was simpler and sufficient.
`NOTEIN_COURSE_<PREFIX>` maps a filename prefix to the exact select option string;
`cargo run -- configure` manages that mapping interactively instead of hand-editing
`.env` (see ROADMAP.md).

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
  existing page titles, the three-step file upload, page creation). Nothing here
  knows about the local filesystem.
- `src/sync.rs`: the diffing logic and `run_sync_cycle`, the only place that imports
  both `notein` and `notion`.

When adding a new external call, put it in the module for that system, not in
`sync.rs` or `main.rs`.

## Comments

Only comment the **why**, never the what. If you'd delete a comment and nothing about
the code becomes less clear, don't add it in the first place.

## Before committing

Run `cargo build`, `cargo test`, and `cargo clippy --all-targets`. Zero warnings, not
just zero errors.
