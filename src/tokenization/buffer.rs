use std::{fs::File, io::Read};
use crate::position::{START_POSITION, FilePosition};

pub trait Buffer {
    fn next(&mut self) -> std::io::Result<u8>;
    fn back(&mut self, count: usize);
    fn range(&self, start: usize, end: usize) -> Vec<u8>;
    fn file(&self) -> &Option<String>;
    fn shift(&self) -> usize;
    fn pos(&self) -> FilePosition;
    fn prev_pos(&self) -> FilePosition;
    fn save_pos(&mut self);
    fn restore_pos(&mut self);
}

pub struct SimpleBuffer {
    storage: Vec<u8>,
    pos: usize,
    saved_pos: Option<usize>,
    file_pos: FilePosition,
    saved_file_pos: Option<FilePosition>,
    prev_file_pos: FilePosition,
    file: Option<String>
}

impl SimpleBuffer {
    pub fn new(data: &[u8], file: Option<String>) -> Self {
        Self {
            storage: Vec::from(data),
            pos: 0,
            saved_pos: None,
            file_pos: START_POSITION,
            saved_file_pos: None,
            prev_file_pos: START_POSITION,
            file: file
        }
    }

    pub fn from_file(filepath: String) -> Result<Self, std::io::Error> {
        let mut data = Vec::new();
        let mut file = File::open(&filepath)?;
        file.read_to_end(&mut data)?;
        Ok(Self::new(&data, Some(filepath)))
    }
}

impl Buffer for SimpleBuffer {
    fn next(&mut self) -> std::io::Result<u8> {
        let result;

        if self.pos >= self.storage.len() {
            result = Ok(0);
        } else {
            result = Ok(self.storage[self.pos]);
            self.prev_file_pos = self.file_pos.clone();
            if self.storage[self.pos] == b'\n' {
                self.file_pos.line += 1;
                self.file_pos.col = 1;
            } else {
                self.file_pos.col += 1;
            }
        }

        self.pos += 1;
        
        result
    }

    fn back(&mut self, count: usize) {
        for _ in 0..count {
            self.pos -= 1;
            if self.pos < self.storage.len() {
                if self.storage[self.pos] != b'\n' {
                    self.file_pos.col -= 1;
                } else {
                    self.file_pos.line -= 1;
                }
            }
        }
    }

    fn range(&self, start: usize, end: usize) -> Vec<u8> {
        let mut actual_end = end;
        if end > self.storage.len() {
            actual_end = self.storage.len();
        }
        Vec::from(&self.storage[start..actual_end])
    }

    fn shift(&self) -> usize {
        self.pos
    }

    fn pos(&self) -> FilePosition {
        self.file_pos.clone()
    }

    fn prev_pos(&self) -> FilePosition {
        self.prev_file_pos.clone()
    }

    fn save_pos(&mut self) {
        self.saved_pos = Some(self.pos);
        self.saved_file_pos = Some(self.file_pos);
    }

    fn restore_pos(&mut self) {
        if self.saved_pos.is_some() {
            self.pos = self.saved_pos.unwrap();
            self.saved_pos = None;
            self.saved_file_pos = None;
        }
    }

    fn file(&self) -> &Option<String> {
        &self.file
    }
}
