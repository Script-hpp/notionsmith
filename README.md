# notionsmith

One-way sync daemon: PDF notes exported from the Notein app -> pages in a single
Notion database, tagged by course.

Notein (com.orion.notein.global) stores notes as `.in` files, plain ZIP archives
containing a Room/SQLite database plus JSON metadata. The handwriting itself is
stored as a proprietary protobuf blob with no public spec. Rather than
reverse-engineering that, this project relies on Notein's native, unpaid "export as
PDF" feature and just watches a folder for the result.

The workflow: export a note as PDF from Notein by hand, name the file
`<PREFIX>_<anything>.pdf` (e.g. `MATHE1_Test1.pdf`), let Syncthing (or any other sync
tool) land it in a watched folder on this machine. `notionsmith` picks it up from
there, uploads it to the one configured Notion database, and sets its `Course`
property to whatever that prefix maps to.

See [CLAUDE.md](CLAUDE.md) for module layout and conventions, and
[ROADMAP.md](ROADMAP.md) for planned work.

## Setup

1. In Notion, create (or reuse) a database with at least a Title property and a
   Files & media property, plus a `Course` select property with one option per course
   you take notes for. Share it with your integration (`Share` -> Connections).
2. `cp .env.example .env` and fill in `NOTEIN_WATCH_DIR`, `NOTION_TOKEN` (an internal
   integration secret from https://www.notion.so/my-integrations), and
   `NOTION_DATABASE_ID`.
3. Run `cargo run -- configure`. It fetches your `Course` options from Notion, suggests
   a filename prefix per course (guaranteeing no two collide), and lets you
   review/edit each one (arrow keys to navigate, Enter to edit, `s` to save). Saving
   writes the confirmed mapping into `.env` as `NOTEIN_COURSE_<PREFIX>` lines, and
   keeps a prefix -> course name reference up to date in two places: a page inside
   your Notion database itself (titled "📋 Notionsmith Prefixes", so it's there on
   your phone in the Notion app regardless of your sync setup), and a plain-text
   `notionsmith-courses.txt` in `NOTEIN_WATCH_DIR` for anyone who also syncs that
   folder elsewhere (e.g. via Syncthing). Either way, naming a file correctly never
   depends on memorizing an abbreviation.
4. `cargo run` to start the sync daemon.

If your Title or Files & media property is named differently than Notion's
defaults, set `NOTION_TITLE_PROPERTY` / `NOTION_FILE_PROPERTY` in `.env` to match
exactly (property names are case-sensitive to the API). `configure`'s prefix
suggestions filter out a small set of German filler words ("und", "der", ...) by
default; override with a comma-separated `NOTEIN_STOPWORDS` for other languages.

## Related

[notionless](https://github.com/Script-hpp/notionless): the other direction, syncing
Notion database pages to Paperless-ngx documents.
