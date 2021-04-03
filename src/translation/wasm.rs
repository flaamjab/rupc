use std::io::Write;

use crate::{semantics::{Type, Types}, tokenization::{Operator, Relation}, translation::output::{Output, TEMPLATE}};

pub struct Wasm {
    output: Output,
    silenced: bool,
}

impl Wasm {
    pub fn new(writer: Box<dyn Write>) -> Self {
        Self {
            silenced: false,
            output: Output::new(writer),
        }
    }

    pub fn mod_start(&mut self) {
        if !self.silenced {
            self.output.write("(module");
            self.output.indent_in();
        }
    }

    pub fn mod_end(&mut self) {
        if !self.silenced {
            self.output.write(")\n");
        }
    }

    pub fn func_import(&mut self, name: &str, types: &Types) {
        if !self.silenced {
            let mut params = String::new();
            for t in types {
                params += &format!("(param {})", self.typename(t))
            }
            
            self.output.writenl(&format!(
                "(func ${} (import \"imports\" \"{}\") {})",
                name, name, params
            ))
        }
    }

    pub fn func_start(&mut self, name: &str, export: bool) {
        if !self.silenced {
            let export_part = if export {
                format!("(export \"{}\")", name)
            } else {
                format!("${}", name)
            };
    
            self.output.writenl(&format!("(func {}", export_part));
            self.output.indent_in();
        }
    }

    pub fn func_local(&mut self, name: &str, type_: &Type) {
        if !self.silenced {
            self.output.write(
                &format!(" (local ${} {})",
                name, self.typename(&type_))
            )
        }
    }

    pub fn func_result(&mut self, type_: &Type) {
        if !self.silenced {
            self.output.write(&format!(" (result {})", self.typename(type_)));
        }
    }
    
    pub fn func_end(&mut self) {
        if !self.silenced {
            self.output.write(")\n");
            self.output.indent_out();
        }
    }

    pub fn constant(&mut self, value: &str, type_: &Type) {
        if !self.silenced {
            self.output.writenl(&format!(
                "{}.const {}",
                self.typename(type_), value
            ))
        }
    }

    pub fn local_set(&mut self, name: &str) {
        if !self.silenced {
            self.output.writenl(&format!("local.set ${}", name));
        }
    }

    pub fn local_get(&mut self, name: &str) {
        if !self.silenced {
            self.output.writenl(&format!("local.get ${}", name));
        }
    }

    pub fn op(&mut self, op: &Operator, type_: &Type) {
        if !self.silenced {
            let cmd = match op {
                Operator::Multiply => "mul",
                Operator::Plus => "add",
                Operator::Minus => "sub",
                Operator::Divide => "div",
                Operator::Or => "or",
                Operator::Xor => "xor",
                _ => todo!("Support other operators")
            };
    
            self.output.writenl(&format!(
                "{}.{}",
                self.typename(type_).to_owned(),
                cmd
            ));
        }
    }

    pub fn relop(&mut self, op: &Relation, type_: &Type) {
        if !self.silenced {
            let cmd = match (op, type_) {
                (Relation::Eq, _) => "eq",
                (Relation::Le, Type::Integer) => "le_s",
                (Relation::Lt, Type::Integer) => "lt_s",
                (Relation::Gt, Type::Integer) => "gt_s",
                (Relation::Ge, Type::Integer) => "ge_s",
                (Relation::Le, Type::Real) => "le",
                (Relation::Lt, Type::Real) => "lt",
                (Relation::Gt, Type::Real) => "gt",
                (Relation::Ge, Type::Real) => "ge",
                (Relation::Ne, _) => "ne",
                _ => todo!("Implement other relation operators")
            };
    
            self.output.writenl(&format!(
                "{}.{}",
                self.typename(type_).to_owned(),
                cmd
            ));
        }
    }

    pub fn eqz(&mut self, type_: &Type) {
        self.output.writenl(&format!(
            "{}.eqz", self.typename(type_)
        ));
    }

    pub fn call(&mut self, name: &str) {
        if !self.silenced {
            self.output.writenl(&format!("call ${}", name));
        }
    }

    pub fn if_start(&mut self) {
        if !self.silenced {
            self.output.writenl("(if");
            self.output.indent_in();
        }
    }

    pub fn then_start(&mut self) {
        if !self.silenced {
            self.output.writenl("(then");
            self.output.indent_in();
        }
    }

    pub fn then_end(&mut self) {
        if !self.silenced {
            self.output.write(")");
            self.output.indent_out();
        }
    }

    pub fn else_start(&mut self) {
        if !self.silenced {
            self.output.writenl("(else");
            self.output.indent_in();
        }
    }

    pub fn else_end(&mut self) {
        if !self.silenced {
            self.output.write(")");
            self.output.indent_out();
        }
    }

    pub fn if_end(&mut self) {
        if !self.silenced {
            self.output.writenl(")");
            self.output.indent_out();
        }
    }

    pub fn loop_start(&mut self, continue_label: &str, end_label: &str) {
        if !self.silenced {
            self.output.writenl(&format!("(block ${}", end_label));
            self.output.indent_in();
            self.output.writenl(&format!("(loop ${}", continue_label));
            self.output.indent_in();
        }
    }

    pub fn br(&mut self, label: &str) {
        if !self.silenced {
            self.output.writenl(&format!("br ${}", label));
        }
    }

    pub fn br_if(&mut self, label: &str) {
        if !self.silenced {
            self.output.writenl(&format!("br_if ${}", label))
        }
    }

    pub fn loop_end(&mut self) {
        if !self.silenced {
            for _ in 0..2 {
                self.output.indent_out();
                self.output.writenl(")");
            }
        }
    }

    pub fn silence(&mut self) {
        if !self.silenced {
            self.silenced = true;
        }
    }

    pub fn fill_nearest_unknown(&mut self, t: &Type) {
        if !self.silenced {
            self.output.fill_last_template(&self.typename(&t));
        }
    }

    fn typename(&self, t: &Type) -> String {
        match t {
            Type::Integer => "i32",
            Type::Real => "f32",
            Type::Scalar(_) => "i32",
            Type::Unknown => {
                TEMPLATE
            },
            _ => unimplemented!("unsupported type")
        }.to_string()
    }
}

impl Drop for Wasm {
    fn drop(&mut self) {
        self.output.flush();
    }
}
