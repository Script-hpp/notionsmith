# notionsmith

Handwritten notes from a phone note-taking app, ending up as searchable pages in
Notion, tagged by course, without ever touching the app's proprietary file format.

notionsmith watches a folder for PDFs exported from [Notein](https://play.google.com/store/apps/details?id=com.orion.notein.global),
uploads each one to a single Notion database, and tags it with a `Course` select
property derived from the filename.

## Why this exists

Getting handwritten notes into Notion could mean fighting Notein's file format to
render the strokes directly. It doesn't need to: Notein already has a native, unpaid
"export as PDF" feature per note, so notionsmith just watches for the
already-rendered PDF and routes it by filename. The one piece of friction left after
that, remembering which of ~30 course prefixes to type when naming a file, gets
solved by `configure`: an interactive TUI that suggests a prefix per course, resolves
collisions between suggestions automatically, and keeps a plain-language reference
(prefix -> course name) as a page inside Notion itself, so it's checkable from the
phone regardless of what else syncs the watch folder.

This is the counterpart to [notionless](https://github.com/Script-hpp/notionless):
notionless takes typed Notion pages out into Paperless-ngx, notionsmith takes
handwritten pages into Notion.

## Status

Honest state, so nobody wastes time:

- **One-way only: local folder -> Notion.** There's no update path back. A note that
  changes in Notion isn't reflected locally, and there's no mechanism for that.
- **Dedup is by title only, not content.** If a file's derived title already exists
  as a page in the database, it's skipped forever, even if the PDF's content
  changed. Re-exporting under the same name does not refresh it in Notion.
- **Single-part file upload only.** Notion's 20 MB cap applies; a bigger PDF needs
  the multi-part upload flow, which isn't implemented.
- **No first-run onboarding.** A missing `.env` just panics with "X must be set"
  instead of dropping into `configure`. See [ROADMAP.md](ROADMAP.md).
- Tested against the Notion API version `2022-06-28`.

## Setup

1. **Create a Notion database** with at least a Title property, a Files & media
   property, and a `Course` select property (one option per course you take notes
   for). Share it with your integration under *Connections*.
2. **Create a Notion integration** at https://www.notion.so/my-integrations and copy
   the *Internal Integration Secret*.

   To find the database ID, open the database as a full page in Notion and look at
   the URL: `https://www.notion.so/<workspace>/<DATABASE_ID>?v=...`. `DATABASE_ID` is
   the 32-character string right before `?v=`.
3. **Configure:**
   ```sh
   cp .env.example .env
   # fill in NOTEIN_WATCH_DIR, NOTION_TOKEN, NOTION_DATABASE_ID
   ```
4. **Map filename prefixes to courses:**
   ```sh
   cargo run -- configure
   ```
   Fetches your `Course` options from Notion, suggests a filename prefix per course
   (guaranteeing no two collide), and lets you review/edit each one (arrow keys to
   navigate, Enter to edit, `s` to save). Saving writes the mapping into `.env` as
   `NOTEIN_COURSE_<PREFIX>` lines, and keeps a prefix -> course reference in two
   places: a page inside the Notion database itself (titled
   "📋 Notionsmith Prefixes"), and a plain-text `notionsmith-courses.txt` in
   `NOTEIN_WATCH_DIR` for anyone who also syncs that folder elsewhere (e.g. via
   Syncthing).
5. **Run:**
   ```sh
   cargo run
   ```

Export a note as PDF from Notein, name the file `<PREFIX>_<anything>.pdf` (e.g.
`MATHE1_Test1.pdf`), and let whatever you use to sync the watch folder (Syncthing or
otherwise) land it there. notionsmith picks it up on the next scan, uploads it, and
sets `Course` to whatever that prefix maps to. The prefix itself is stripped from the
page title, since the course already carries that information.

## Configuration

| Variable | Required | Meaning |
| --- | --- | --- |
| `NOTEIN_WATCH_DIR` | yes | Folder that gets filled with exported PDFs |
| `NOTION_TOKEN` | yes | Notion Internal Integration Secret |
| `NOTION_DATABASE_ID` | yes | The single database every note is uploaded to |
| `NOTION_TITLE_PROPERTY` | no | Title property name (default: `Name`) |
| `NOTION_FILE_PROPERTY` | no | Files & media property name (default: `Files & media`) |
| `NOTION_COURSE_PROPERTY` | no | Course select property name (default: `Course`) |
| `NOTION_STATUS_PROPERTY` / `NOTION_STATUS_VALUE` | no | Also set a Status select value on every imported note; both or neither |
| `NOTEIN_STOPWORDS` | no | Comma-separated filler words `configure` ignores when abbreviating a course name (default: a small German list) |
| `SYNC_INTERVAL_SECS` | no | Seconds between scans of the watch folder (default: `60`) |
| `NOTEIN_COURSE_<PREFIX>` | yes (one per course) | Maps a filename prefix to an exact `Course` select option; managed by `configure`, not meant to be hand-written |

notionsmith looks for a `.env` file in two places, first one found wins:
`~/.config/notionsmith/.env` (or `$XDG_CONFIG_HOME/notionsmith/.env`), then `.env` in
the current directory.

## Project layout

- `src/main.rs`: entry point, env config, the outer poll loop, `configure` dispatch.
- `src/notein.rs`: watch-folder scanning and the `<PREFIX>_` filename convention.
- `src/notion.rs` + `src/notion/model.rs`: all Notion API interaction (file upload,
  page creation, the reference page).
- `src/sync.rs`: the diffing logic and `run_sync_cycle`.
- `src/configure.rs`: the interactive `configure` TUI (ratatui/crossterm) and its
  prefix-suggestion logic.

See [CLAUDE.md](CLAUDE.md) for conventions and [ROADMAP.md](ROADMAP.md) for planned
work.

## License

MIT, see [LICENSE](LICENSE).
