use std::{collections::LinkedList, fmt::Debug};

use crate::semantics::Fields;

pub type Enumeration = LinkedList<String>;
pub type Types = LinkedList<Type>;

#[derive(Clone, PartialEq)]
pub enum Type {
    Record(Fields),
    Scalar(Enumeration),
    Integer,
    Real,
    Char,
    Unknown
}

pub fn boolean() -> Type{
    Type::Scalar([ 
        "false".to_string(),
        "true".to_string()
    ].iter().cloned().collect())
}

impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self {
            Type::Record(_) => "Record",
            Type::Scalar(_) => "Scalar",
            Type::Integer => "Integer",
            Type::Real => "Real",
            Type::Char => "Char",
            Type::Unknown => "Unknown",
        };

        write!(f, "{}", t)
    }
}
