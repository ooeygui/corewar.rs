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
}
