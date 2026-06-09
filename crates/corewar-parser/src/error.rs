use thiserror::Error;

/// Errors that can occur during parsing of Redcode source.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("syntax error at line {line}: {message}")]
    Syntax { line: usize, message: String },

    #[error("undefined label '{label}' at line {line}")]
    UndefinedLabel { label: String, line: usize },

    #[error("duplicate label '{label}' at line {line}")]
    DuplicateLabel { label: String, line: usize },

    #[error("warrior exceeds maximum length of {max_length} instructions")]
    TooLong { max_length: usize },

    #[error("invalid opcode '{opcode}' at line {line}")]
    InvalidOpcode { opcode: String, line: usize },

    #[error("invalid modifier '{modifier}' at line {line}")]
    InvalidModifier { modifier: String, line: usize },

    #[error("invalid directive '{directive}' at line {line}")]
    InvalidDirective { directive: String, line: usize },

    #[error("invalid expression '{expression}' at line {line}")]
    InvalidExpression { expression: String, line: usize },

    #[error("circular EQU definition involving '{name}' at line {line}")]
    CircularEqu { name: String, line: usize },

    #[error("division by zero in expression at line {line}")]
    DivisionByZero { line: usize },
}

impl ParseError {
    pub(crate) fn syntax(line: usize, message: impl Into<String>) -> Self {
        Self::Syntax {
            line,
            message: message.into(),
        }
    }
}
