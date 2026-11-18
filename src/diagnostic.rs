//! Lint diagnostics, shared so that every consumer (CLI, editor backend)
//! handles every language's lint output the same way.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "error",
            Level::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    /// 1-indexed line the diagnostic applies to (as a whole).
    pub line: usize,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Error,
            message: message.into(),
            line,
        }
    }

    pub fn warning(message: impl Into<String>, line: usize) -> Self {
        Diagnostic {
            level: Level::Warning,
            message: message.into(),
            line,
        }
    }
}
