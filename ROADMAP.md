# Roadmap

The biggest risk to this tool isn't the sync logic, it's abandonment: if setting up a
new subject means hand-copying a 32-character database id into `.env`, that friction
is enough to make the whole thing fall out of use, and notes end up unsorted again.
Every item below exists to keep the day-to-day use as close to zero-config as
possible.

## 1. Interactive `configure` command (next up)

Replace manual `.env` editing with a guided CLI wizard:

- `notionsmith configure` calls `POST /v1/search` (filtered to `object: "database"`)
  with the configured `NOTION_TOKEN`, listing every database the integration can see.
- The databases are printed as a numbered list (title plus id) so the user can pick
  one interactively instead of copying ids out of a browser URL bar.
- For each selected database, the wizard asks for the filename prefix it should map
  to (e.g. `MATHE1`) and confirms the database has a Title property and a Files &
  media property before accepting it, catching the "property does not exist" class of
  error (see the `files and media` casing issue from initial setup) before it ever
  reaches a sync cycle.
- Selections are written back into `.env` (or created if missing) as
  `NOTEIN_DB_<PREFIX>=<id>`, preserving comments and unrelated keys already there.
- Re-running `configure` shows currently mapped prefixes first and lets the user add
  to them, rather than starting over.

This turns "add a new subject" into one command with a menu, no manual id lookups.

## 2. First-run onboarding

On startup, if `.env` is missing entirely, drop straight into `configure` instead of
just printing "NOTEIN_WATCH_DIR must be set" and exiting. The watch folder and Notion
token get asked for as the first two wizard steps.

## 3. Unmapped-prefix reminder, not just a log line

Right now an unmapped prefix (`MATHE_TEST.pdf` when only `NOTEIN_DB_MATHE1` exists)
is a skip logged once per cycle and easy to miss. Track prefixes seen with no mapping
across cycles and surface them once, clearly, as "these files are waiting: run
`configure` to map them", instead of a line buried in a 60-second poll loop.

## 4. Optional desktop notification on successful upload

A quiet confirmation ("MATHE1_Test1.pdf -> Notion") when a page is created, so the
daemon's success doesn't only exist as scrollback in a terminal nobody is watching.
