pub const START_POSITION: FilePosition = FilePosition { line: 1, col: 1 };

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct FilePosition {
    pub line: usize,
    pub col: usize,
}

impl FilePosition {
    pub fn new(line: usize, col: usize) -> Self {
        FilePosition { line: line, col: col }
    }
}
