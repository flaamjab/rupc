use std::{collections::LinkedList, io::{BufWriter, Write}};

pub const TEMPLATE: &str = "UNKNOWN";

pub struct Output {
    indent: usize,
    parts: Vec<String>,
    template_indices: LinkedList<usize>,
    writer: BufWriter<Box<dyn Write>>,
}

impl Output {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            indent: 0,
            parts: Vec::with_capacity(16),
            template_indices: LinkedList::new(),
            writer: BufWriter::new(writer),
        }
    }

    pub fn indent_in(&mut self) {
        self.indent += 2;
    }

    pub fn indent_out(&mut self) {
        self.indent -= 2;
    }

    pub fn indent_reset(&mut self) {
        self.indent = 0;
    }

    pub fn writenl(&mut self, msg: &str) {
        let indent = " ".repeat(self.indent);
        self.write(&format!("\n{}{}", indent, msg));
    }

    pub fn write(&mut self, msg: &str) {
        if msg.contains(TEMPLATE) {
            self.template_indices.push_back(self.parts.len());
        }

        self.parts.push(msg.to_string());
    }

    pub fn fill_last_template(&mut self, with: &str) {
        let maybe_index = self.template_indices.back();
        if let Some(&index) = maybe_index {
            let part = &self.parts[index];
            self.parts[index] = part.replace(TEMPLATE, with);
            self.template_indices.pop_back();
        }
    }

    pub fn flush(&mut self) {
        for p in &self.parts {
            self.writer.write_fmt(format_args!("{}", p))
            .unwrap_or_else(|e| {
                panic!("IO error occurred when generating code: {}", e);
            });
        }
        self.parts.clear();
        self.template_indices.clear();
    }
}
