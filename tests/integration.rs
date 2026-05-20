use duck_diag::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TestCode {
  SyntaxError,
  TypeMismatch,
  UnusedVar,
}

impl DiagnosticCode for TestCode {
  fn code(&self) -> &str {
    match self {
      Self::SyntaxError => "E0001",
      Self::TypeMismatch => "E0002",
      Self::UnusedVar => "W0001",
    }
  }
  fn severity(&self) -> Severity {
    match self {
      Self::UnusedVar => Severity::Warning,
      _ => Severity::Error,
    }
  }
}

#[test]
fn engine_counts_errors_and_warnings() {
  let source = "let x = 1;\nlet y = 2;\nlet z = 3;";
  let mut engine = DiagnosticEngine::<TestCode>::new();

  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "bad syntax")
      .with_label(Label::primary(Span::new("test.rs", 1, 4, 1), Some("here".into()))),
  );
  engine.emit(
    Diagnostic::new(TestCode::UnusedVar, "unused variable `y`")
      .with_label(Label::primary(Span::new("test.rs", 2, 4, 1), Some("never read".into()))),
  );
  engine.emit(
    Diagnostic::new(TestCode::TypeMismatch, "expected int, found string")
      .with_label(Label::primary(Span::new("test.rs", 3, 8, 1), Some("wrong type".into()))),
  );

  engine.print_all(source);

  assert_eq!(engine.error_count(), 2);
  assert_eq!(engine.warning_count(), 1);
  assert!(engine.has_errors());
}

#[test]
fn engine_clear_resets_everything() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(Diagnostic::new(TestCode::SyntaxError, "err"));
  engine.clear();

  assert_eq!(engine.error_count(), 0);
  assert_eq!(engine.warning_count(), 0);
  assert!(!engine.has_errors());
  assert!(engine.get_diagnostics().is_empty());
}

#[test]
fn diagnostic_builder_chain() {
  let source = "let val = foo + bar;";
  let d = Diagnostic::new(TestCode::SyntaxError, "unexpected token")
    .with_label(Label::primary(Span::new("test.rs", 1, 10, 3), Some("here".into())))
    .with_label(Label::secondary(Span::new("test.rs", 1, 16, 3), Some("related".into())))
    .with_note("check your syntax")
    .with_help("did you mean `let`?");

  let formatter = DiagnosticFormatter::new(&d, source);
  print!("{}", formatter.format());

  assert_eq!(d.labels.len(), 2);
  assert_eq!(d.notes.len(), 1);
  assert!(d.help.is_some());
  assert_eq!(d.severity, Severity::Error);
}

#[test]
fn plain_format_contains_code_and_message() {
  let source = "let x = ;";
  let d = Diagnostic::new(TestCode::SyntaxError, "unexpected semicolon")
    .with_label(Label::primary(Span::new("test.rs", 1, 8, 1), None));

  let formatter = DiagnosticFormatter::new(&d, source);
  let plain = formatter.format_plain();

  println!("{}", plain);

  assert!(plain.contains("E0001"));
  assert!(plain.contains("unexpected semicolon"));
  assert!(plain.contains("test.rs:1:8"));
}

#[test]
fn colored_format_contains_code_and_message() {
  let source = "let x = ;";
  let d = Diagnostic::new(TestCode::SyntaxError, "unexpected semicolon")
    .with_label(Label::primary(Span::new("test.rs", 1, 8, 1), None));

  let formatter = DiagnosticFormatter::new(&d, source);
  let colored = formatter.format();

  print!("{}", colored);

  assert!(colored.contains("unexpected semicolon"));
}

#[test]
fn format_all_plain_includes_summary() {
  let source = "line1\nline2";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "err1")
      .with_label(Label::primary(Span::new("test.rs", 1, 0, 5), Some("first error".into()))),
  );
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "err2")
      .with_label(Label::primary(Span::new("test.rs", 2, 0, 5), Some("second error".into()))),
  );

  let output = engine.format_all_plain(source);
  println!("{}", output);

  assert!(output.contains("2 previous errors"));
}

#[test]
fn warning_only_summary() {
  let source = "let _unused = 42;";
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::UnusedVar, "unused variable")
      .with_label(Label::primary(
        Span::new("test.rs", 1, 4, 7),
        Some("declared but never used".into()),
      ))
      .with_help("remove or prefix with `_`"),
  );

  engine.print_all(source);

  let output = engine.format_all_plain(source);
  assert!(output.contains("1 warning emitted"));
  assert!(!engine.has_errors());
}

#[test]
fn span_new_constructor() {
  let span = Span::new("file.rs", 10, 5, 3);
  assert_eq!(&*span.file, "file.rs");
  assert_eq!(span.line, 10);
  assert_eq!(span.column, 5);
  assert_eq!(span.length, 3);
}

#[test]
fn label_styles() {
  let span = Span::new("f.rs", 1, 0, 1);
  let primary = Label::primary(span.clone(), Some("msg".into()));
  let secondary = Label::secondary(span, None::<String>);

  assert_eq!(primary.style, LabelStyle::Primary);
  assert!(primary.message.is_some());
  assert_eq!(secondary.style, LabelStyle::Secondary);
  assert!(secondary.message.is_none());
}

#[test]
fn multiple_labels_same_line() {
  let source = "let result = a + b * c;";
  let mut engine = DiagnosticEngine::<TestCode>::new();

  engine.emit(
    Diagnostic::new(TestCode::TypeMismatch, "mismatched types in expression")
      .with_label(Label::primary(Span::new("expr.rs", 1, 13, 1), Some("this is a string".into())))
      .with_label(Label::secondary(Span::new("expr.rs", 1, 17, 1), Some("this is an int".into())))
      .with_note("cannot add `String` and `i32`")
      .with_help("convert one side: `a.parse::<i32>()`"),
  );

  engine.print_all(source);

  assert_eq!(engine.error_count(), 1);
}

#[test]
fn from_zero_based_adds_one() {
  let s = Span::from_zero_based("a.rs", 0, 0, 1);
  assert_eq!(s.line, 1);
  assert_eq!(s.column, 1);
  let s2 = Span::from_zero_based("a.rs", 5, 12, 3);
  assert_eq!(s2.line, 6);
  assert_eq!(s2.column, 13);
}

#[test]
fn from_zero_based_saturates_on_usize_max() {
  let s = Span::from_zero_based("a.rs", usize::MAX, usize::MAX, 0);
  assert_eq!(s.line, usize::MAX);
  assert_eq!(s.column, usize::MAX);
}

#[test]
fn label_with_note_attaches_inline_note() {
  let span = Span::new("a.rs", 1, 1, 3);
  let l = Label::primary(span, Some("hi".into())).with_note("see also: rule X");
  assert_eq!(l.note.as_deref(), Some("see also: rule X"));
}

#[test]
fn suggestion_renders_in_pretty_output() {
  let source = "var x = 1;";
  let span = Span::new("a.js", 1, 1, 3);
  let d = Diagnostic::new(TestCode::SyntaxError, "prefer `let`")
    .with_label(Label::primary(span.clone(), Some("here".into())))
    .with_suggestion(
      Suggestion::new(span, "let")
        .with_message("replace `var`")
        .with_applicability(Applicability::MachineApplicable),
    );
  let f = DiagnosticFormatter::new(&d, source);
  let out = f.format_plain();
  // header line has the suggestion message
  assert!(out.contains("help: replace `var`"));
  // diff lines: minus original + plus rewritten
  assert!(out.contains("- var x = 1;"));
  assert!(out.contains("+ let x = 1;"));
  assert!(out.contains("auto-applicable"));
}

#[test]
fn render_options_color_disable_matches_plain() {
  let source = "let x = 1;";
  let d = Diagnostic::new(TestCode::SyntaxError, "bad")
    .with_label(Label::primary(Span::new("a.rs", 1, 5, 1), None));
  let f1 = DiagnosticFormatter::new(&d, source)
    .with_options(RenderOptions { color: false, ..Default::default() });
  let f2 = DiagnosticFormatter::new(&d, source);
  assert_eq!(f1.format(), f2.format_plain());
}

#[test]
fn source_cache_reused_across_diagnostics() {
  let source = "a\nb\nc\n";
  let cache = SourceCache::new(source);
  let d1 = Diagnostic::new(TestCode::SyntaxError, "x")
    .with_label(Label::primary(Span::new("f", 1, 1, 1), None));
  let d2 = Diagnostic::new(TestCode::SyntaxError, "y")
    .with_label(Label::primary(Span::new("f", 2, 1, 1), None));
  let f1 = DiagnosticFormatter::with_cache(&d1, &cache);
  let f2 = DiagnosticFormatter::with_cache(&d2, &cache);
  assert!(f1.format_plain().contains("x"));
  assert!(f2.format_plain().contains("y"));
}

#[test]
fn bug_severity_counts_separately() {
  #[derive(Debug, Clone, Copy)]
  struct Ice;
  impl DiagnosticCode for Ice {
    fn code(&self) -> &str {
      "ICE0001"
    }
    fn severity(&self) -> Severity {
      Severity::Bug
    }
  }
  let mut engine = DiagnosticEngine::<Ice>::new();
  engine.emit(Diagnostic::new(Ice, "internal compiler error"));
  assert_eq!(engine.bug_count(), 1);
  assert!(engine.has_bugs());
  assert_eq!(engine.error_count(), 0);
}

#[test]
fn url_appears_in_pretty_output() {
  #[derive(Debug, Clone, Copy)]
  struct WithUrl;
  impl DiagnosticCode for WithUrl {
    fn code(&self) -> &str {
      "L0001"
    }
    fn severity(&self) -> Severity {
      Severity::Warning
    }
    fn url(&self) -> Option<&'static str> {
      Some("https://example.com/L0001")
    }
  }
  let d =
    Diagnostic::new(WithUrl, "lint").with_label(Label::primary(Span::new("a.rs", 1, 1, 1), None));
  let f = DiagnosticFormatter::new(&d, "x");
  assert!(f.format_plain().contains("https://example.com/L0001"));
}

#[test]
fn multi_file_renders_two_sections() {
  let source = "abc\ndef\n";
  let d = Diagnostic::new(TestCode::SyntaxError, "cross-file mismatch")
    .with_label(Label::primary(Span::new("a.rs", 1, 1, 3), Some("here".into())))
    .with_label(Label::secondary(Span::new("b.rs", 2, 1, 3), Some("and here".into())));
  let f = DiagnosticFormatter::new(&d, source);
  let out = f.format_plain();
  assert!(out.contains("a.rs:1:1"));
  assert!(out.contains("b.rs:2:1"));
}

#[test]
fn tab_padding_aligns_caret() {
  // tabs in source → caret should appear at expanded column, not raw byte column
  let source = "\t\tfoo";
  let d = Diagnostic::new(TestCode::SyntaxError, "x")
    .with_label(Label::primary(Span::new("a", 1, 3, 3), None));
  let f = DiagnosticFormatter::new(&d, source).with_options(RenderOptions {
    color: false,
    tab_width: 4,
    ..Default::default()
  });
  let out = f.format();
  // Expanded: 8 spaces (2 tabs at width 4) before "foo".
  // Caret should appear at column 8 (display) → the line containing the
  // underline must start with at least 8 leading spaces after the gutter.
  assert!(out.contains("        ^^^"));
}

#[test]
fn diag_macro_compiles() {
  let span = Span::new("a", 1, 1, 1);
  let d = duck_diag::diag!(TestCode::SyntaxError, span, "bad");
  assert_eq!(d.message, "bad");
  assert_eq!(d.labels.len(), 1);
}

#[cfg(feature = "json")]
#[test]
fn json_output_includes_required_fields() {
  use serde::Serialize;
  #[derive(Debug, Clone, Copy, Serialize)]
  enum C {
    X,
  }
  impl DiagnosticCode for C {
    fn code(&self) -> &str {
      "X0001"
    }
    fn severity(&self) -> Severity {
      Severity::Error
    }
  }
  let mut engine = DiagnosticEngine::<C>::new();
  engine.emit(
    Diagnostic::new(C::X, "boom")
      .with_label(Label::primary(Span::new("a.rs", 1, 1, 1), Some("here".into()))),
  );
  let json = engine.format_all_json();
  assert!(json.contains("\"code\""));
  assert!(json.contains("\"X0001\""));
  assert!(json.contains("\"severity\""));
  assert!(json.contains("\"error\""));
  assert!(json.contains("\"labels\""));
}

#[test]
fn error_with_notes_and_help() {
  let source = "fn foo() {\n  return 42;\n}";
  let mut engine = DiagnosticEngine::<TestCode>::new();

  engine.emit(
    Diagnostic::new(TestCode::TypeMismatch, "mismatched return type")
      .with_label(Label::primary(
        Span::new("main.rs", 2, 9, 2),
        Some("expected `()`, found `i32`".into()),
      ))
      .with_note("function `foo` has no return type annotation")
      .with_note("implicit return type is `()`")
      .with_help("add a return type: `fn foo() -> i32`"),
  );

  engine.print_all(source);

  assert_eq!(engine.error_count(), 1);
}

#[test]
fn compact_renders_header_and_location_without_source() {
  let d = Diagnostic::new(TestCode::SyntaxError, "bad token")
    .with_label(Label::primary(Span::new("a.rs", 7, 3, 5), Some("here".into())))
    .with_note("see rule X")
    .with_help("rename it");

  let out = format_compact(&d, false);
  assert!(out.contains("error[E0001]: bad token"));
  assert!(out.contains("--> a.rs:7:3"));
  assert!(out.contains("= primary: here"));
  assert!(out.contains("= note: see rule X"));
  assert!(out.contains("= help: rename it"));
  assert!(!out.contains(" | "));
}

#[test]
fn compact_groups_locations_by_file() {
  let d = Diagnostic::new(TestCode::TypeMismatch, "cross-file mismatch")
    .with_label(Label::primary(Span::new("a.rs", 1, 1, 1), None::<String>))
    .with_label(Label::secondary(Span::new("b.rs", 5, 2, 1), None::<String>));

  let out = format_compact(&d, false);
  assert!(out.contains("--> a.rs:1:1"));
  assert!(out.contains("--> b.rs:5:2"));
}

#[test]
fn compact_engine_format_all_includes_summary() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "boom")
      .with_label(Label::primary(Span::new("a.rs", 1, 1, 1), None::<String>)),
  );
  engine.emit(
    Diagnostic::new(TestCode::UnusedVar, "unused")
      .with_label(Label::primary(Span::new("a.rs", 2, 1, 1), None::<String>)),
  );

  let out = engine.format_all_compact_plain();
  assert!(out.contains("error[E0001]: boom"));
  assert!(out.contains("warning[W0001]: unused"));
  assert!(out.contains("could not compile due to 1 previous error"));
  assert!(out.contains("1 warning emitted"));
}

#[test]
fn compact_method_matches_free_fn() {
  let d = Diagnostic::new(TestCode::SyntaxError, "x")
    .with_label(Label::primary(Span::new("a.rs", 1, 1, 1), Some("y".into())));
  let via_method = d.format_compact(false);
  let via_fn = format_compact(&d, false);
  assert_eq!(via_method, via_fn);
}

#[test]
fn smart_renders_compact_when_file_missing() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(Diagnostic::new(TestCode::SyntaxError, "boom").with_label(Label::primary(
    Span::new("/path/that/does/not/exist.rs", 5, 3, 1),
    None::<String>,
  )));
  let out = format_all_smart(&engine, false);
  assert!(out.contains("error[E0001]: boom"));
  assert!(out.contains("--> /path/that/does/not/exist.rs:5:3"));
  assert!(!out.contains(" | "));
}

#[test]
fn smart_falls_back_compact_for_synthetic_span() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "config issue")
      .with_label(Label::primary(Span::synthetic("<config>"), None::<String>)),
  );
  let out = format_all_smart(&engine, false);
  assert!(out.contains("error[E0001]: config issue"));
  assert!(!out.contains(" | "));
}

#[test]
fn smart_includes_summary() {
  let mut engine = DiagnosticEngine::<TestCode>::new();
  engine.emit(
    Diagnostic::new(TestCode::SyntaxError, "bad")
      .with_label(Label::primary(Span::synthetic("x"), None::<String>)),
  );
  let out = format_all_smart(&engine, false);
  assert!(out.contains("could not compile due to 1 previous error"));
}
