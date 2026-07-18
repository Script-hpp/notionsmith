use crate::notion;
use crossterm::event::{ self, Event, KeyCode, KeyEventKind };
use crossterm::terminal::{ EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode };
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{ Constraint, Direction, Layout };
use ratatui::style::{ Color, Modifier, Style };
use ratatui::text::{ Line, Span };
use ratatui::widgets::{ Block, Borders, List, ListItem, ListState, Paragraph };
use std::io::{ self, Stdout };
use std::path::{ Path, PathBuf };

/// One course from Notion, paired with the filename prefix that will route to it.
struct CourseRow {
    name: String,
    prefix: String,
}

/// Converts a trailing roman numeral (as its own word, I to X) to a digit string.
/// Only single module names in this curriculum go past III, but X covers any
/// plausible course numbering.
fn roman_to_digit(word: &str) -> Option<&'static str> {
    match word {
        "I" => Some("1"),
        "II" => Some("2"),
        "III" => Some("3"),
        "IV" => Some("4"),
        "V" => Some("5"),
        "VI" => Some("6"),
        "VII" => Some("7"),
        "VIII" => Some("8"),
        "IX" => Some("9"),
        "X" => Some("10"),
        _ => None,
    }
}

/// Filler words that shouldn't count as their own initial when abbreviating a
/// multi-word course name (e.g. "Kommunikations- und Netztechnik" should not
/// abbreviate "und" into the initials). Defaults to German since that's the
/// maintainer's curriculum, but this tool isn't German-specific: override with a
/// comma-separated `NOTEIN_STOPWORDS` env var for any other language.
fn default_stopwords() -> std::collections::HashSet<String> {
    ["und", "der", "die", "das", "des", "dem", "im", "am"].iter().map(|word| word.to_string()).collect()
}

/// Loads the stopword list from `NOTEIN_STOPWORDS` (comma-separated) if set,
/// otherwise falls back to `default_stopwords`.
fn load_stopwords() -> std::collections::HashSet<String> {
    match std::env::var("NOTEIN_STOPWORDS") {
        Ok(value) if !value.trim().is_empty() => {
            value
                .split(',')
                .map(|word| word.trim().to_lowercase())
                .filter(|word| !word.is_empty())
                .collect()
        }
        _ => default_stopwords(),
    }
}

/// Suggests a filename prefix for a Notion course name using `letters_per_word`
/// letters from each significant word, e.g. with 1 letter "Theoretische
/// Informatik I" -> "TI1"; with 2 letters -> "THIN1". Single-word names always use
/// their first five letters, since there's only one word to vary. Higher
/// `letters_per_word` values are only used to break a collision between two
/// course names that would otherwise suggest the same prefix (see
/// `disambiguate_prefixes`); the starting suggestion always uses 1.
fn prefix_with_letters(course_name: &str, letters_per_word: usize, stopwords: &std::collections::HashSet<String>) -> String {
    let words: Vec<&str> = course_name.split_whitespace().collect();

    let (base_words, suffix): (Vec<&str>, String) = match words.split_last() {
        Some((last, rest)) if !rest.is_empty() => {
            match roman_to_digit(last) {
                Some(digit) => (rest.to_vec(), digit.to_string()),
                None => (words.clone(), String::new()),
            }
        }
        _ => (words.clone(), String::new()),
    };

    let significant: Vec<&str> = base_words
        .iter()
        .copied()
        .filter(|word| !stopwords.contains(&word.to_lowercase()))
        .collect();
    let significant = if significant.is_empty() { base_words } else { significant };

    let prefix = if significant.len() > 1 {
        significant
            .iter()
            .map(|word| word.chars().take(letters_per_word).collect::<String>())
            .collect::<String>()
            .to_uppercase()
    } else {
        significant
            .first()
            .map(|word| word.chars().take(5).collect::<String>().to_uppercase())
            .unwrap_or_default()
    };

    format!("{}{}", prefix, suffix)
}

/// Suggests a filename prefix for a Notion course name, e.g. "Mathematik I" ->
/// "MATHE1", "Theoretische Informatik I" -> "TI1". This is only a starting point:
/// the user reviews and can edit every suggestion in the TUI before saving, and
/// `disambiguate_prefixes` resolves any collision between two suggestions before
/// the TUI even opens.
fn suggest_prefix(course_name: &str, stopwords: &std::collections::HashSet<String>) -> String {
    prefix_with_letters(course_name, 1, stopwords)
}

/// Resolves collisions between suggested prefixes in place: two course names that
/// initially suggest the same prefix (e.g. "Theoretische Informatik I" and
/// "Technische Informatik I" both naively suggest "TI1") would otherwise silently
/// overwrite each other as `NOTEIN_COURSE_<PREFIX>` env vars. Whichever row comes
/// first keeps the short suggestion; every later collision is regenerated with more
/// letters per word until it's unique.
fn disambiguate_prefixes(rows: &mut [CourseRow], stopwords: &std::collections::HashSet<String>) {
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

    for row in rows.iter_mut() {
        if used.insert(row.prefix.clone()) {
            continue;
        }

        let mut letters_per_word = 2;
        loop {
            let candidate = prefix_with_letters(&row.name, letters_per_word, stopwords);
            if !used.contains(&candidate) || letters_per_word >= 8 {
                row.prefix = candidate;
                break;
            }
            letters_per_word += 1;
        }
        used.insert(row.prefix.clone());
    }
}

/// Prefixes shared by more than one row, e.g. after a manual edit in the TUI
/// re-introduces a collision `disambiguate_prefixes` already resolved once. Saving
/// with a duplicate is refused, since it would silently overwrite one course's env
/// var with another's.
fn find_duplicate_prefixes(rows: &[CourseRow]) -> Vec<String> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut duplicates: Vec<String> = Vec::new();

    for row in rows {
        if !seen.insert(&row.prefix) && !duplicates.contains(&row.prefix) {
            duplicates.push(row.prefix.clone());
        }
    }

    duplicates
}

/// Where `configure` writes `NOTEIN_COURSE_<PREFIX>` lines to: the XDG config path
/// if one already exists there (matching how `main::load_config` prefers it),
/// otherwise `.env` in the current directory.
fn resolve_env_path() -> PathBuf {
    let xdg_path = std::env
        ::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|xdg| !xdg.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("notionsmith").join(".env"));

    match xdg_path {
        Some(path) if path.is_file() => path,
        _ => PathBuf::from(".env"),
    }
}

fn strip_quotes(value: &str) -> &str {
    let is_quoted = value.len() >= 2 && {
        let bytes = value.as_bytes();
        (bytes[0] == b'"' && bytes[value.len() - 1] == b'"') ||
            (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
    };
    if is_quoted { &value[1..value.len() - 1] } else { value }
}

/// Reads existing `NOTEIN_COURSE_<PREFIX>=<course name>` lines from the env file (if
/// any), so a course that was already mapped keeps its current prefix as the
/// starting point instead of being overwritten by a fresh suggestion.
fn read_existing_prefixes(env_path: &Path) -> std::collections::HashMap<String, String> {
    let mut course_to_prefix = std::collections::HashMap::new();
    let Ok(content) = std::fs::read_to_string(env_path) else {
        return course_to_prefix;
    };

    for line in content.lines() {
        let Some(rest) = line.trim().strip_prefix("NOTEIN_COURSE_") else { continue };
        let Some((prefix, value)) = rest.split_once('=') else { continue };
        course_to_prefix.insert(strip_quotes(value).to_string(), prefix.to_string());
    }

    course_to_prefix
}

fn quote_if_needed(value: &str) -> String {
    if value.contains(' ') { format!("\"{}\"", value) } else { value.to_string() }
}

/// Sorted "PREFIX -> Course name" lines, shared by both the local cheat sheet and
/// the Notion reference page.
fn reference_lines(rows: &[CourseRow]) -> Vec<String> {
    let mut sorted: Vec<&CourseRow> = rows.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted.into_iter().map(|row| format!("{} -> {}", row.prefix, row.name)).collect()
}

const CHEAT_SHEET_FILENAME: &str = "notionsmith-kurse.txt";

/// Writes a plain-text "prefix -> course name" reference into the watch folder
/// itself.
///
/// The point isn't to memorize ~30 cryptic abbreviations, it's to never have to:
/// for anyone who happens to sync that folder to their phone (e.g. via Syncthing),
/// this file lands right there too. `upsert_reference_page` below is the version
/// that works for everyone regardless of how the watch folder gets there.
fn write_cheat_sheet(watch_dir: &Path, lines: &[String]) -> io::Result<()> {
    let mut content = String::from("Notionsmith Kurs-Präfixe\n\nDateien benennen als: [PREFIX]_[Thema]_[Datum].pdf\n\n");
    for line in lines {
        content.push_str(line);
        content.push('\n');
    }

    std::fs::write(watch_dir.join(CHEAT_SHEET_FILENAME), content)
}

/// Writes the confirmed prefix mapping back to the env file: existing
/// `NOTEIN_COURSE_` lines are dropped and replaced with a fresh block, everything
/// else in the file is left untouched.
fn write_course_map(env_path: &Path, rows: &[CourseRow]) -> io::Result<()> {
    let existing = std::fs::read_to_string(env_path).unwrap_or_default();
    let mut kept_lines: Vec<&str> = existing
        .lines()
        .filter(|line| !line.trim().starts_with("NOTEIN_COURSE_"))
        .collect();
    while kept_lines.last().is_some_and(|line| line.trim().is_empty()) {
        kept_lines.pop();
    }

    let mut output = kept_lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str("\n# Course prefixes (generated by `notionsmith configure`)\n");
    for row in rows {
        output.push_str(&format!("NOTEIN_COURSE_{}={}\n", row.prefix, quote_if_needed(&row.name)));
    }

    std::fs::write(env_path, output)
}

/// Restores the terminal on drop, including on an early return or panic inside
/// `run`, so a crash doesn't leave the user's shell stuck in raw/alternate-screen
/// mode.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

enum Mode {
    Browsing,
    Editing(String),
}

fn draw(frame: &mut ratatui::Frame, rows: &[CourseRow], selected: usize, mode: &Mode, status: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let preview_prefix = match mode {
        Mode::Editing(buffer) => buffer.as_str(),
        Mode::Browsing => rows.get(selected).map(|row| row.prefix.as_str()).unwrap_or(""),
    };
    let today = chrono_like_today();
    let preview = Paragraph::new(
        format!("Speichere Notein-Exporte als [PREFIX]_[Thema]_[DATUM].pdf, z.B. {}_Thema_{}.pdf", preview_prefix, today)
    ).block(Block::default().borders(Borders::ALL).title("Namenskonvention"));
    frame.render_widget(preview, chunks[0]);

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let prefix_display = match mode {
                Mode::Editing(buffer) if i == selected => format!("{}_", buffer),
                _ => row.prefix.clone(),
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12}", prefix_display), Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::raw(&row.name)
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Kurse (Notion Kurs-Property)"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    let help = match mode {
        Mode::Browsing => "↑/↓ auswählen   Enter Präfix bearbeiten   s speichern & beenden   q beenden ohne speichern",
        Mode::Editing(_) => "Tippen zum Ändern   Enter bestätigen   Esc abbrechen",
    };
    let footer = Paragraph::new(format!("{}\n{}", help, status)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[2]);
}

/// Minimal `YYYY-MM-DD` for today, without pulling in a date/time crate for a single
/// preview string.
fn chrono_like_today() -> String {
    use std::time::{ SystemTime, UNIX_EPOCH };
    let days_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() / 86400).unwrap_or(0);

    // Civil-from-days (Howard Hinnant's algorithm), avoids a chrono dependency for
    // one cosmetic date in a preview string.
    let z = (days_since_epoch as i64) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Runs the interactive `configure` TUI: fetch the Notion `Kurs` select options,
/// let the user review/edit a suggested filename prefix for each, and write the
/// confirmed mapping to the env file as `NOTEIN_COURSE_<PREFIX>` lines.
pub async fn run(
    client: &reqwest::Client,
    notion_token: &str,
    database_id: &str,
    course_property: &str,
    title_property: &str,
    watch_dir: &Path
) -> Result<(), Box<dyn std::error::Error>> {
    let course_names = notion::fetch_course_options(client, notion_token, database_id, course_property).await?;
    if course_names.is_empty() {
        println!("No options found on the '{}' select property; add some in Notion first.", course_property);
        return Ok(());
    }

    let stopwords = load_stopwords();
    let env_path = resolve_env_path();
    let existing_prefixes = read_existing_prefixes(&env_path);

    let mut rows: Vec<CourseRow> = course_names
        .into_iter()
        .map(|name| {
            let prefix = existing_prefixes.get(&name).cloned().unwrap_or_else(|| suggest_prefix(&name, &stopwords));
            CourseRow { name, prefix }
        })
        .collect();
    // Existing mappings can themselves collide (e.g. a corrupted env file from
    // before this check existed), so this runs unconditionally, not just for fresh
    // suggestions.
    disambiguate_prefixes(&mut rows, &stopwords);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let mut guard = TerminalGuard { terminal };

    let mut selected = 0usize;
    let mut mode = Mode::Browsing;
    let mut status = String::new();
    let mut saved = false;

    loop {
        guard.terminal.draw(|frame| draw(frame, &rows, selected, &mode, &status))?;

        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match &mut mode {
            Mode::Browsing => {
                match key.code {
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        selected = (selected + 1).min(rows.len().saturating_sub(1));
                    }
                    KeyCode::Enter => {
                        mode = Mode::Editing(rows[selected].prefix.clone());
                    }
                    KeyCode::Char('s') => {
                        let duplicates = find_duplicate_prefixes(&rows);
                        if !duplicates.is_empty() {
                            status = format!(
                                "⚠ Doppelte Präfixe, erst beheben: {}. Nicht gespeichert.",
                                duplicates.join(", ")
                            );
                            continue;
                        }

                        write_course_map(&env_path, &rows)?;
                        let lines = reference_lines(&rows);
                        let mut messages = vec![format!("Gespeichert nach {}.", env_path.display())];

                        match
                            notion::upsert_reference_page(client, notion_token, database_id, title_property, &lines).await
                        {
                            Ok(()) =>
                                messages.push(
                                    format!("Referenz-Seite '{}' in Notion aktualisiert.", notion::REFERENCE_PAGE_TITLE)
                                ),
                            Err(e) => messages.push(format!("Referenz-Seite in Notion fehlgeschlagen: {}.", e)),
                        }

                        if let Err(e) = write_cheat_sheet(watch_dir, &lines) {
                            messages.push(format!("Lokale Kurzreferenz fehlgeschlagen: {}.", e));
                        }

                        status = messages.join(" ");
                        saved = true;
                        break;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
            Mode::Editing(buffer) => {
                match key.code {
                    KeyCode::Enter => {
                        let new_prefix = buffer.trim().to_uppercase();
                        if !new_prefix.is_empty() {
                            rows[selected].prefix = new_prefix;
                        }
                        mode = Mode::Browsing;
                    }
                    KeyCode::Esc => {
                        mode = Mode::Browsing;
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                        buffer.push(c);
                    }
                    _ => {}
                }
            }
        }
    }

    drop(guard);

    if saved {
        println!("{}", status);
    } else {
        println!("Abgebrochen, nichts gespeichert.");
    }

    Ok(())
}

#[cfg(test)]
#[path = "configure/tests.rs"]
mod tests;
