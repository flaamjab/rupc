use std::collections::HashMap;
use crate::semantics::{Type, Types};

pub type Fields = HashMap<String, Type>;

#[derive(Clone)]
pub enum Identifier {
    Variable(String, Type),
    Type(Type),
    Procedure(Types),
    Unknown
}
