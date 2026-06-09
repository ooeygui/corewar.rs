//! Lexer/tokenizer for Redcode source files.
//! Handles comments, labels, directives, and metadata.

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

#[cfg(not(feature = "std"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use corewar_core::Opcode;

use crate::error::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetadataKind {
    Name,
    Author,
    Strategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataDirective {
    pub(crate) line: usize,
    pub(crate) kind: MetadataKind,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatementKind {
    Instruction,
    Org,
    End,
    Equ,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatementLine {
    pub(crate) line: usize,
    pub(crate) label: Option<String>,
    pub(crate) statement: String,
    pub(crate) kind: StatementKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LexedLine {
    Metadata(MetadataDirective),
    Statement(StatementLine),
}

pub(crate) fn lex(source: &str) -> Result<Vec<LexedLine>, ParseError> {
    let mut lines = Vec::new();
    let mut pending_label: Option<(usize, String)> = None;

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if let Some(metadata) = parse_metadata(trimmed, line_number) {
            lines.push(LexedLine::Metadata(metadata));
            continue;
        }

        let code = strip_comment(raw_line).trim();
        if code.is_empty() {
            continue;
        }

        if let Some(label) = parse_standalone_label(code) {
            if pending_label.is_some() {
                return Err(ParseError::syntax(
                    line_number,
                    "multiple standalone labels before a statement",
                ));
            }
            pending_label = Some((line_number, label));
            continue;
        }

        let mut statement = classify_statement(code, line_number)?;
        if let Some((_, label)) = pending_label.take() {
            if statement.label.is_some() {
                return Err(ParseError::syntax(
                    line_number,
                    "multiple labels before a statement",
                ));
            }
            statement.label = Some(label);
        }

        lines.push(LexedLine::Statement(statement));
    }

    if let Some((line, _)) = pending_label {
        return Err(ParseError::syntax(line, "label without statement"));
    }

    Ok(lines)
}

fn parse_metadata(line: &str, line_number: usize) -> Option<MetadataDirective> {
    let comment = line.trim_start().strip_prefix(';')?.trim_start();

    for (prefix, kind) in [
        ("name", MetadataKind::Name),
        ("author", MetadataKind::Author),
        ("strategy", MetadataKind::Strategy),
    ] {
        if comment.len() < prefix.len() {
            continue;
        }

        let (head, tail) = comment.split_at(prefix.len());
        if head.eq_ignore_ascii_case(prefix)
            && tail.chars().next().map_or(true, char::is_whitespace)
        {
            return Some(MetadataDirective {
                line: line_number,
                kind,
                value: tail.trim().to_string(),
            });
        }
    }

    None
}

fn strip_comment(line: &str) -> &str {
    match line.find(';') {
        Some(index) => &line[..index],
        None => line,
    }
}

fn parse_standalone_label(code: &str) -> Option<String> {
    let (label, rest) = split_word(code);
    if label.is_empty() || !rest.trim().is_empty() {
        return None;
    }

    if is_opcode(label)
        || is_directive(label, "ORG")
        || is_directive(label, "END")
        || is_directive(label, "EQU")
    {
        return None;
    }

    let normalized = normalize_label(label);
    is_valid_label(&normalized).then_some(normalized)
}

fn is_valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$'))
}

fn classify_statement(code: &str, line: usize) -> Result<StatementLine, ParseError> {
    let (first, first_rest) = split_word(code);
    if first.is_empty() {
        return Err(ParseError::syntax(line, "expected statement"));
    }

    if is_directive(first, "ORG") {
        return Ok(StatementLine {
            line,
            label: None,
            statement: code.trim().to_string(),
            kind: StatementKind::Org,
        });
    }

    if is_directive(first, "END") {
        return Ok(StatementLine {
            line,
            label: None,
            statement: code.trim().to_string(),
            kind: StatementKind::End,
        });
    }

    if is_opcode(first) {
        return Ok(StatementLine {
            line,
            label: None,
            statement: code.trim().to_string(),
            kind: StatementKind::Instruction,
        });
    }

    let (second, _) = split_word(first_rest);
    let kind = if is_directive(second, "EQU") {
        StatementKind::Equ
    } else if is_directive(second, "ORG") {
        StatementKind::Org
    } else if is_directive(second, "END") {
        StatementKind::End
    } else {
        StatementKind::Instruction
    };

    Ok(StatementLine {
        line,
        label: Some(normalize_label(first)),
        statement: first_rest.trim_start().to_string(),
        kind,
    })
}

fn split_word(input: &str) -> (&str, &str) {
    let input = input.trim_start();
    if input.is_empty() {
        return ("", "");
    }

    match input.find(char::is_whitespace) {
        Some(index) => (&input[..index], &input[index..]),
        None => (input, ""),
    }
}

fn normalize_label(label: &str) -> String {
    label.trim_end_matches(':').to_ascii_uppercase()
}

fn is_opcode(value: &str) -> bool {
    let opcode = value.split_once('.').map_or(value, |(opcode, _)| opcode);
    Opcode::from_str(opcode).is_ok()
}

fn is_directive(value: &str, directive: &str) -> bool {
    value.eq_ignore_ascii_case(directive)
}

use core::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_extracts_metadata_and_statements() {
        let lines = lex(";name Imp\nstart MOV 0, 1 ; comment\nstep EQU 4\nORG start\n").unwrap();

        assert!(matches!(lines[0], LexedLine::Metadata(_)));
        assert!(matches!(lines[1], LexedLine::Statement(_)));
        assert!(matches!(lines[2], LexedLine::Statement(_)));
        assert!(matches!(lines[3], LexedLine::Statement(_)));
    }
}
