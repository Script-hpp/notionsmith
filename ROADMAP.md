# Roadmap

The biggest risk to this tool isn't the sync logic, it's abandonment: if using it
day-to-day means remembering cryptic abbreviations or hand-editing `.env`, that
friction is enough to make the whole thing fall out of use, and notes end up
unsorted again. Every item below exists to keep the day-to-day use as close to
zero-effort as possible.

## Done: interactive `configure` TUI

`cargo run -- configure` (see `src/configure.rs`) fetches the `Kurs` select options
from the one Notion database, suggests a filename prefix per course, lets the user
review/edit every suggestion in a full-screen ratatui TUI, and writes the confirmed
mapping into `.env` as `NOTEIN_COURSE_<PREFIX>` lines.

Two things this had to get right, both learned from a real failure while building
the first version:

- **Collisions must be caught, not just avoided.** The first version's suggestion
  algorithm gave "Theoretische Informatik I" and "Technische Informatik I" the exact
  same prefix (`TI1`), silently overwriting one course's mapping with the other's
  when both landed in the same `.env`. `disambiguate_prefixes` now resolves this
  automatically before the TUI even opens, and saving is refused outright
  (`find_duplicate_prefixes`) if a manual edit reintroduces a collision.
- **Memorization was never the right ask.** No one can reliably recall ~30 generated
  abbreviations (`MUTDKI`, `SUVS`, ...) well enough to type them correctly on a
  phone. Instead of chasing a "more memorable" algorithm, `configure` writes a
  plain-text prefix -> course name reference (`notionsmith-kurse.txt`) directly into
  `NOTEIN_WATCH_DIR`. Since that folder already syncs to the phone via Syncthing,
  the cheat sheet is right there to check, exactly when and where naming a file
  actually happens.

## Next up

## 1. First-run onboarding

On startup, if `.env` is missing entirely, drop straight into `configure` instead of
just printing "NOTEIN_WATCH_DIR must be set" and exiting. The watch folder and Notion
token get asked for as the first two wizard steps (they're not currently collected
by `configure` at all, only the course mapping is).

## 2. Unmapped-prefix reminder, not just a log line

Right now an unmapped prefix (`MATHE_TEST.pdf` when only `NOTEIN_COURSE_MATHE1`
exists) is a skip logged once per cycle and easy to miss. Track prefixes seen with
no mapping across cycles and surface them once, clearly, as "these files are
waiting: run `configure` to map them", instead of a line buried in a 60-second poll
loop.

## 3. Optional desktop notification on successful upload

A quiet confirmation ("MATHE1_Test1.pdf -> Notion") when a page is created, so the
daemon's success doesn't only exist as scrollback in a terminal nobody is watching.
