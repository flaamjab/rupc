use std::fmt::{Display, Formatter};
use std::error::Error;
use crate::position::FilePosition;

#[derive(Debug, Clone)]
pub enum CompilationErrorKind {
    LexicalError,
    SyntaxError,
    SemanticError,
}

#[derive(Debug, Clone)]
pub struct CompilationError {
    kind: CompilationErrorKind,
    pos: FilePosition,
    path: Option<String>,
    msg: String
}

impl CompilationError {
    pub fn new(
        kind: CompilationErrorKind,
        path: &Option<String>,
        pos: FilePosition,
        msg: &str
    ) -> Self {
        CompilationError {
            kind: kind,
            path: path.clone(),
            pos: pos.clone(),
            msg: String::from(msg),
        }
    }

    pub fn kind(&self) -> CompilationErrorKind {
        self.kind.clone()
    }

    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn pos(&self) -> FilePosition {
        self.pos.clone()
    }

}

impl Error for CompilationError {}

impl Display for CompilationError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let path = self.path.clone().unwrap_or("~".into());
        write!(
            f, "{:?} at {}:{}:{}: {}",
            self.kind, path,
            self.pos.line, self.pos.col, self.msg
        )
    }
}