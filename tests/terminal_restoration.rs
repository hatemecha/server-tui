//! Terminal restoration is validated by the RAII guard design.
//! Full raw-mode enter/leave is not safe inside automatic CI without a PTY.

#[test]
fn terminal_module_exports_guard() {
    // Smoke: type exists and module links. Interactive restore is covered manually
    // per TESTING.md (q / Ctrl+C / panic hook).
    let name = std::any::type_name::<server_tui::terminal::TerminalGuard>();
    assert!(name.contains("TerminalGuard"));
}
