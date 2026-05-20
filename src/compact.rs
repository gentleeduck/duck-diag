//! Source-less ("compact") rendering. When the caller doesn't have the
//! original source string at hand — log shippers, batch CI summaries, LSP
//! tools that already display source themselves — render the header,
//! location, label messages, notes, help, and suggestions **without**
//! the line snippet + caret art.
//!
//! Layout (one diagnostic):
//!
//! ```text
//! error[E0001]: unterminated jsx tag (see https://example.com/E0001)
//!   --> samples/index.mdx:14:3
//!   = primary: tag opened here, never closed
//!   = note: jsx tags must close before end of file
//!   = help: add `</Callout>` or self-close with `/>`
//!   = help: try this: replace with self-closing form
//!       <Callout title="hi" />
//! ```
//!
//! Multi-file diagnostics get one `--> file:L:C` line per distinct file.

use colored::*;

use crate::diagnostic::{Applicability, Diagnostic, DiagnosticCode, Label, LabelStyle, Suggestion};
use crate::formatter::sanitize_for_display;
use crate::style::*;

/// Render a single diagnostic in compact (source-less) form.
pub fn format_compact<C: DiagnosticCode>(d: &Diagnostic<C>, color: bool) -> String {
  let mut out = String::new();
  write_header(d, &mut out, color);
  write_locations(d, &mut out, color);
  write_label_messages(d, &mut out, color);
  write_notes_help(d, &mut out, color);
  write_suggestions(d, &mut out, color);
  out.push('\n');
  out
}

fn write_header<C: DiagnosticCode>(d: &Diagnostic<C>, out: &mut String, color: bool) {
  out.push_str(&format!(
    "{}[{}]: {}",
    severity_word(d.severity, color),
    code_word(d.severity, d.code.code(), color),
    sanitize_for_display(&d.message, false),
  ));
  if let Some(u) = d.code.url() {
    out.push_str(&format!(" {}", paint(&format!("(see {u})"), color, |s| s.blue().italic())));
  }
  out.push('\n');
}

fn write_locations<C: DiagnosticCode>(d: &Diagnostic<C>, out: &mut String, color: bool) {
  if d.labels.is_empty() {
    return;
  }
  let arrow_s = arrow(color);
  for file in distinct_files(&d.labels) {
    let Some(anchor) = anchor_for_file(&d.labels, &file) else { continue };
    let loc = if color {
      format!(
        "{}:{}:{}",
        anchor.span.file.white().bold(),
        anchor.span.line.to_string().white().bold(),
        anchor.span.column.to_string().white().bold(),
      )
    } else {
      format!("{}:{}:{}", anchor.span.file, anchor.span.line, anchor.span.column)
    };
    out.push_str(&format!("  {} {}\n", arrow_s, loc));
  }
}

fn write_label_messages<C: DiagnosticCode>(d: &Diagnostic<C>, out: &mut String, color: bool) {
  let eq = eq_sep(color);
  for l in &d.labels {
    if let Some(msg) = &l.message {
      let kind = match l.style {
        LabelStyle::Primary => "primary",
        LabelStyle::Secondary => "note",
      };
      out.push_str(&format!(
        "  {} {}: {}\n",
        eq,
        paint_label(d.severity, l.style, kind, color),
        paint_label(d.severity, l.style, &sanitize_for_display(msg, false), color),
      ));
    }
    if let Some(note) = &l.note {
      out.push_str(&format!(
        "       {}\n",
        paint(&sanitize_for_display(note, false), color, |s| s.cyan().italic()),
      ));
    }
  }
}

fn write_notes_help<C: DiagnosticCode>(d: &Diagnostic<C>, out: &mut String, color: bool) {
  let eq = eq_sep(color);
  for note in &d.notes {
    out.push_str(&format!(
      "  {} {}: {}\n",
      eq,
      meta_label("note", color),
      sanitize_for_display(note, false),
    ));
  }
  if let Some(help) = &d.help {
    out.push_str(&format!(
      "  {} {}: {}\n",
      eq,
      meta_label("help", color),
      sanitize_for_display(help, false),
    ));
  }
}

fn write_suggestions<C: DiagnosticCode>(d: &Diagnostic<C>, out: &mut String, color: bool) {
  if d.suggestions.is_empty() {
    return;
  }
  let eq = eq_sep(color);
  let help = meta_label("help", color);
  for s in &d.suggestions {
    let header = s.message.clone().unwrap_or_else(|| "try this:".to_string());
    out.push_str(&format!("  {} {}: {}\n", eq, help, sanitize_for_display(&header, false)));
    for line in s.replacement.lines() {
      let line = sanitize_for_display(line, false);
      out.push_str(&format!("       {}\n", paint(&line, color, |s| s.green())));
    }
    write_applicability(out, s, color);
  }
}

fn write_applicability(out: &mut String, s: &Suggestion, color: bool) {
  let kind = match s.applicability {
    Applicability::MachineApplicable => "auto-applicable",
    Applicability::MaybeIncorrect => "review needed",
    Applicability::HasPlaceholders => "has placeholders",
    Applicability::Unspecified => return,
  };
  out.push_str(&format!("       ({})\n", paint(kind, color, |s| s.dimmed())));
}

fn distinct_files(labels: &[Label]) -> Vec<std::sync::Arc<str>> {
  let mut files: Vec<std::sync::Arc<str>> = Vec::new();
  for l in labels {
    if !files.iter().any(|f| **f == *l.span.file) {
      files.push(l.span.file.clone());
    }
  }
  files
}

fn anchor_for_file<'a>(labels: &'a [Label], file: &str) -> Option<&'a Label> {
  labels
    .iter()
    .filter(|l| *l.span.file == *file)
    .find(|l| l.style == LabelStyle::Primary)
    .or_else(|| labels.iter().find(|l| *l.span.file == *file))
}
