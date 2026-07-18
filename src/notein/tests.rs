use super::*;

#[test]
fn parses_prefix_from_typical_filename() {
    assert_eq!(parse_prefix("MATHE1_Test1.pdf"), Some("MATHE1".to_string()));
}

#[test]
fn uppercases_lowercase_prefix() {
    assert_eq!(parse_prefix("mathe1_Test1.pdf"), Some("MATHE1".to_string()));
}

#[test]
fn accepts_uppercase_pdf_extension() {
    assert_eq!(parse_prefix("MATHE1_Test1.PDF"), Some("MATHE1".to_string()));
}

#[test]
fn rejects_filename_without_underscore() {
    assert_eq!(parse_prefix("Test1.pdf"), None);
}

#[test]
fn rejects_filename_without_pdf_extension() {
    assert_eq!(parse_prefix("MATHE1_Test1.txt"), None);
}

#[test]
fn rejects_filename_starting_with_underscore() {
    assert_eq!(parse_prefix("_Test1.pdf"), None);
}

#[test]
fn keeps_remaining_underscores_in_the_stem() {
    assert_eq!(parse_prefix("MATHE1_Test_1_final.pdf"), Some("MATHE1".to_string()));
}

