# notionsmith

One-way sync daemon: PDF notes exported from the Notein app -> pages in Notion
databases, one database per subject prefix.

Notein (com.orion.notein.global) stores notes as `.in` files, plain ZIP archives
containing a Room/SQLite database plus JSON metadata. The handwriting itself is
stored as a proprietary protobuf blob with no public spec. Rather than
reverse-engineering that, this project relies on Notein's native, unpaid "export as
PDF" feature and just watches a folder for the result.

The workflow: export a note as PDF from Notein by hand, name the file
`<PREFIX>_<anything>.pdf` (e.g. `MATHE1_Test1.pdf`), let Syncthing (or any other sync
tool) land it in a watched folder on this machine. `notionsmith` picks it up from
there and uploads it to the Notion database configured for that prefix.

See [CLAUDE.md](CLAUDE.md) for module layout and conventions, and
[ROADMAP.md](ROADMAP.md) for planned work, most importantly an interactive
`configure` command to replace manual `.env` editing.

## Setup

1. `cp .env.example .env` and fill in:
   - `NOTEIN_WATCH_DIR`: the folder your sync tool fills with exported PDFs.
   - `NOTION_TOKEN`: an internal integration secret from
     https://www.notion.so/my-integrations.
   - `NOTEIN_DB_<PREFIX>`: one line per subject, mapping the filename prefix to a
     Notion database id.
2. In each target database, share it with your integration (`Share` -> Connections),
   and make sure it has a Title property and a Files & media property. If either is
   named differently than Notion's defaults, set `NOTION_TITLE_PROPERTY` /
   `NOTION_FILE_PROPERTY` in `.env` to match exactly (property names are
   case-sensitive to the API).
3. `cargo run`.

## Related

[notionless](https://github.com/Script-hpp/notionless): the other direction, syncing
Notion database pages to Paperless-ngx documents.
