use server_tui::sanitize::{sanitize_cell, sanitize_text, truncate_width};

#[test]
fn strips_ansi_and_controls() {
    let raw = "\x1b[1;31malert\x1b[0m\x07";
    let s = sanitize_text(raw);
    assert!(!s.contains('\x1b'));
    assert!(!s.contains('\x07'));
    assert!(s.contains("alert"));
}

#[test]
fn unicode_names_survive() {
    let name = "servicio-ñandú.service";
    assert!(sanitize_cell(name).contains("ñandú"));
}

#[test]
fn truncate_respects_budget() {
    assert_eq!(truncate_width("abcdef", 3), "abc");
}

#[test]
fn truncates_unicode_without_panic() {
    let s = "ñ".repeat(200);
    let out = truncate_width(&s, 120);
    assert!(out.chars().count() <= 120);
}
