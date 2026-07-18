use super::*;

#[test]
fn abbreviates_single_word_course_with_roman_numeral() {
    assert_eq!(suggest_prefix("Mathematik I"), "MATHE1");
}

#[test]
fn abbreviates_single_word_course_without_roman_numeral() {
    assert_eq!(suggest_prefix("Datenbanksysteme"), "DATEN");
}

#[test]
fn distinguishes_a_second_numbered_module_from_the_first() {
    assert_eq!(suggest_prefix("Datenbanksysteme II"), "DATEN2");
}

#[test]
fn uses_initials_for_multi_word_course_with_roman_numeral() {
    assert_eq!(suggest_prefix("Theoretische Informatik I"), "TI1");
}

#[test]
fn uses_initials_for_multi_word_course_without_roman_numeral() {
    assert_eq!(suggest_prefix("Anwendungsprojekt Informatik"), "AI");
}

#[test]
fn ignores_german_filler_words_when_abbreviating() {
    assert_eq!(suggest_prefix("Kommunikations- und Netztechnik"), "KN");
}

#[test]
fn theoretische_and_technische_informatik_collide_at_one_letter() {
    // The real bug this test guards against: both suggest "TI1" on their own,
    // which is exactly why `disambiguate_prefixes` has to run before saving.
    assert_eq!(suggest_prefix("Theoretische Informatik I"), "TI1");
    assert_eq!(suggest_prefix("Technische Informatik I"), "TI1");
}

#[test]
fn disambiguate_prefixes_keeps_the_first_occurrence_short() {
    let mut rows = vec![
        CourseRow { name: "Theoretische Informatik I".to_string(), prefix: suggest_prefix("Theoretische Informatik I") },
        CourseRow { name: "Technische Informatik I".to_string(), prefix: suggest_prefix("Technische Informatik I") }
    ];
    disambiguate_prefixes(&mut rows);

    assert_eq!(rows[0].prefix, "TI1");
    assert_ne!(rows[1].prefix, "TI1");
}

#[test]
fn disambiguate_prefixes_produces_no_duplicates() {
    let mut rows = vec![
        CourseRow { name: "Theoretische Informatik I".to_string(), prefix: suggest_prefix("Theoretische Informatik I") },
        CourseRow { name: "Technische Informatik I".to_string(), prefix: suggest_prefix("Technische Informatik I") },
        CourseRow { name: "Theoretische Informatik II".to_string(), prefix: suggest_prefix("Theoretische Informatik II") },
        CourseRow { name: "Technische Informatik II".to_string(), prefix: suggest_prefix("Technische Informatik II") }
    ];
    disambiguate_prefixes(&mut rows);

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
