use super::*;

fn stopwords() -> std::collections::HashSet<String> {
    default_stopwords()
}

#[test]
fn abbreviates_single_word_course_with_roman_numeral() {
    assert_eq!(suggest_prefix("Mathematik I", &stopwords()), "MATHE1");
}

#[test]
fn abbreviates_single_word_course_without_roman_numeral() {
    assert_eq!(suggest_prefix("Datenbanksysteme", &stopwords()), "DATEN");
}

#[test]
fn distinguishes_a_second_numbered_module_from_the_first() {
    assert_eq!(suggest_prefix("Datenbanksysteme II", &stopwords()), "DATEN2");
}

#[test]
fn uses_initials_for_multi_word_course_with_roman_numeral() {
    assert_eq!(suggest_prefix("Theoretische Informatik I", &stopwords()), "TI1");
}

#[test]
fn uses_initials_for_multi_word_course_without_roman_numeral() {
    assert_eq!(suggest_prefix("Anwendungsprojekt Informatik", &stopwords()), "AI");
}

#[test]
fn ignores_filler_words_when_abbreviating() {
    assert_eq!(suggest_prefix("Kommunikations- und Netztechnik", &stopwords()), "KN");
}

#[test]
fn custom_stopwords_override_the_default_german_list() {
    let custom: std::collections::HashSet<String> = ["of", "the"].iter().map(|w| w.to_string()).collect();
    assert_eq!(suggest_prefix("Theory of the Machines", &custom), "TM");
}

#[test]
fn theoretische_and_technische_informatik_collide_at_one_letter() {
    // The real bug this test guards against: both suggest "TI1" on their own,
    // which is exactly why `disambiguate_prefixes` has to run before saving.
    assert_eq!(suggest_prefix("Theoretische Informatik I", &stopwords()), "TI1");
    assert_eq!(suggest_prefix("Technische Informatik I", &stopwords()), "TI1");
}

#[test]
fn disambiguate_prefixes_keeps_the_first_occurrence_short() {
    let words = stopwords();
    let mut rows = vec![
        CourseRow {
            name: "Theoretische Informatik I".to_string(),
            prefix: suggest_prefix("Theoretische Informatik I", &words),
        },
        CourseRow { name: "Technische Informatik I".to_string(), prefix: suggest_prefix("Technische Informatik I", &words) }
    ];
    disambiguate_prefixes(&mut rows, &words);

    assert_eq!(rows[0].prefix, "TI1");
    assert_ne!(rows[1].prefix, "TI1");
}

#[test]
fn disambiguate_prefixes_produces_no_duplicates() {
    let words = stopwords();
    let mut rows = vec![
        CourseRow {
            name: "Theoretische Informatik I".to_string(),
            prefix: suggest_prefix("Theoretische Informatik I", &words),
        },
        CourseRow { name: "Technische Informatik I".to_string(), prefix: suggest_prefix("Technische Informatik I", &words) },
        CourseRow {
            name: "Theoretische Informatik II".to_string(),
            prefix: suggest_prefix("Theoretische Informatik II", &words),
        },
        CourseRow { name: "Technische Informatik II".to_string(), prefix: suggest_prefix("Technische Informatik II", &words) }
    ];
    disambiguate_prefixes(&mut rows, &words);

    assert!(find_duplicate_prefixes(&rows).is_empty());
}

#[test]
fn find_duplicate_prefixes_reports_untouched_collisions() {
    let rows = vec![
        CourseRow { name: "Theoretische Informatik I".to_string(), prefix: "TI1".to_string() },
        CourseRow { name: "Technische Informatik I".to_string(), prefix: "TI1".to_string() }
    ];
    assert_eq!(find_duplicate_prefixes(&rows), vec!["TI1".to_string()]);
}

#[test]
fn find_duplicate_prefixes_is_empty_for_unique_rows() {
    let rows = vec![
        CourseRow { name: "Mathematik I".to_string(), prefix: "MATHE1".to_string() },
        CourseRow { name: "Mathematik II".to_string(), prefix: "MATHE2".to_string() }
    ];
    assert!(find_duplicate_prefixes(&rows).is_empty());
}

#[test]
fn reference_lines_are_sorted_by_course_name() {
    let rows = vec![
        CourseRow { name: "Theoretische Informatik I".to_string(), prefix: "TI1".to_string() },
        CourseRow { name: "Mathematik I".to_string(), prefix: "MATHE1".to_string() }
    ];
    assert_eq!(reference_lines(&rows), vec!["MATHE1 -> Mathematik I".to_string(), "TI1 -> Theoretische Informatik I".to_string()]);
}

#[test]
fn strips_matching_double_quotes() {
    assert_eq!(strip_quotes("\"Mathematik I\""), "Mathematik I");
}

#[test]
fn strips_matching_single_quotes() {
    assert_eq!(strip_quotes("'Mathematik I'"), "Mathematik I");
}

#[test]
fn leaves_unquoted_value_untouched() {
    assert_eq!(strip_quotes("Mathematik"), "Mathematik");
}

#[test]
fn quotes_values_with_spaces() {
    assert_eq!(quote_if_needed("Mathematik I"), "\"Mathematik I\"");
}

#[test]
fn leaves_single_word_values_unquoted() {
    assert_eq!(quote_if_needed("Mathematik"), "Mathematik");
}
