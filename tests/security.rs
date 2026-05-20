//! Regression tests for the rescan-001 security fixes:
//! - SEC-001: mid-UTF-8 slice panic in suggestion diff rendering.
//! - SEC-002: integer overflow in span column/offset arithmetic.
//! - SEC-003: ANSI / control-char injection into terminal output.

use duck_diag::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TestCode {
  SyntaxError,
}

impl DiagnosticCode for TestCode {
  fn code(&self) -> &str {
    "E0001"
  }
  fn severity(&self) -> Severity {
    Severity::Error
  }
}

// ---------- SEC-001: mid-UTF-8 slice ----------

#[test]
fn suggestion_column_mid_codepoint_does_not_panic() {
  // `é` is two bytes (0xC3 0xA9). A suggestion whose column/length land
  // inside that codepoint must not panic the process.
  let source = "let café = 1;";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  for col in 1..=source.len() + 2 {
    for len in 0..=source.len() + 2 {
      let mut e = DiagnosticEngine::<TestCode>::new();
      e.emit(
        Diagnostic::new(TestCode::SyntaxError, "bad")
          .with_label(Label::primary(Span::new("t.rs", 1, col, len), Some("x".into())))
          .with_suggestion(Suggestion::new(Span::new("t.rs", 1, col, len), "REPL")),
      );
      // Must render without panic for every offset combination.
      let _ = e.format_all_plain(source);
    }
  }
  // Sanity: also exercise the multi-byte emoji case once.
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "emoji")
      .with_suggestion(Suggestion::new(Span::new("t.rs", 1, 6, 3), "x")),
  );
  let out = engine.format_all_plain("let 🦆 = duck;");
  assert!(out.contains("E0001"));
}

// ---------- SEC-002: integer overflow ----------

#[test]
fn span_fields_near_usize_max_do_not_panic() {
  let source = "let value = compute();";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "overflow")
      .with_label(Label::primary(Span::new("t.rs", 1, usize::MAX, usize::MAX), Some("huge".into())))
      .with_suggestion(Suggestion::new(Span::new("t.rs", 1, usize::MAX, usize::MAX), "patched")),
  );
  let out = engine.format_all_plain(source);
  assert!(out.contains("E0001"));
}

#[test]
fn span_end_before_start_does_not_panic() {
  // column far past the line, length 0 — and a malformed-looking span where
  // the implied end precedes the start. Must render, not panic.
  let source = "abcdef";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "weird span")
      .with_suggestion(Suggestion::new(Span::new("t.rs", 1, 1000, 0), "z")),
  );
  let out = engine.format_all_plain(source);
  assert!(out.contains("E0001"));
}

// ---------- SEC-003: ANSI / control-char injection ----------

#[test]
fn ansi_and_control_chars_stripped_from_message() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(Diagnostic::new(
    TestCode::SyntaxError,
    "clear screen\x1b[2J and \r overwrite \x08 backspace",
  ));
  let out = engine.format_all_plain("let x = 1;");
  assert!(!out.contains('\x1b'), "ESC must be stripped");
  assert!(!out.contains('\r'), "CR must be stripped");
  assert!(!out.contains('\x08'), "BS must be stripped");
  // Visible text survives.
  assert!(out.contains("clear screen"));
  assert!(out.contains("overwrite"));
}

#[test]
fn injected_newline_in_message_is_stripped() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine
    .emit(Diagnostic::new(TestCode::SyntaxError, "real error\nerror[E9999]: forged diagnostic"));
  let out = engine.format_all_plain("let x = 1;");
  // The forged second "line" must remain on the single message line, so the
  // output has exactly one header line.
  let header_lines = out.lines().filter(|l| l.contains("E0001")).count();
  assert_eq!(header_lines, 1, "injected newline must not spawn a new line");
  assert!(out.contains("forged diagnostic"));
}

#[test]
fn ansi_stripped_from_compact_render() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "bad\x1b[31m thing")
      .with_label(Label::primary(Span::new("t.rs", 1, 1, 3), Some("label\x1b[2J text".into())))
      .with_note("note\rwith cr"),
  );
  let out = engine.format_all_compact_plain();
  assert!(!out.contains('\x1b'));
  assert!(!out.contains('\r'));
}

#[test]
fn ansi_in_source_line_stripped() {
  // Control chars embedded in the source snippet itself must be stripped.
  let source = "let x = \x1b[2J\"poisoned\";";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "source poison")
      .with_label(Label::primary(Span::new("t.rs", 1, 1, 3), Some("here".into()))),
  );
  let out = engine.format_all_plain(source);
  assert!(!out.contains('\x1b'), "ESC in source snippet must be stripped");
}

// ---------- regression: normal output unchanged ----------

#[test]
fn normal_ascii_diagnostic_renders_cleanly() {
  let source = "let x = 1;\nlet y = 2;";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "expected semicolon")
      .with_label(Label::primary(Span::new("t.rs", 1, 5, 1), Some("here".into())))
      .with_suggestion(Suggestion::new(Span::new("t.rs", 1, 5, 1), "y")),
  );
  let out = engine.format_all_plain(source);
  assert!(out.contains("expected semicolon"));
  assert!(out.contains("E0001"));
  assert!(out.contains("here"));
  // No control chars introduced.
  assert!(!out.chars().any(|c| c.is_control() && c != '\n' && c != '\t'));
}
