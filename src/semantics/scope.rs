use std::{boxed::Box, collections::{HashMap}, error::Error, fmt::Display};

use crate::semantics::{Identifier, Type, boolean};

pub type Identifiers = HashMap<String, Identifier>;

#[derive(Clone)]
pub struct Scope {
    outer_scope: Option<Box<Scope>>,
    identifiers: Identifiers,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new(
            [
                ("char".to_string(), Identifier::Type(Type::Char)),
                ("integer".to_string(), Identifier::Type(Type::Integer)),
                ("real".to_string(), Identifier::Type(Type::Real)),
                ("boolean".to_string(), Identifier::Type(boolean())),
                ("writeln_int".to_string(), Identifier::Procedure(
                    [
                        Type::Integer
                    ].iter().cloned().collect()
                )),
                ("writeln_real".to_string(), Identifier::Procedure(
                    [
                        Type::Real
                    ].iter().cloned().collect()
                ))
            ].iter().cloned().collect(),
        )
    }
}

impl Scope {
    pub fn new(table: Identifiers) -> Self {
        Scope {
            outer_scope: None,
            identifiers: table,
        }
    }

    pub fn with_outer(
        scope: Box<Scope>,
        identifiers: Identifiers
    ) -> Box<Self> {
        Box::new(Scope {
            outer_scope: Some(scope),
            identifiers,
        })
    }

    pub fn empty_with_outer(scope: Box<Scope>) -> Box<Self> {
        Self::with_outer(scope, Identifiers::new())
    }

    pub fn collapse(self) -> Option<Box<Self>> {
        self.outer_scope
    }

    pub fn put(
        &mut self,
        name: String,
        id: Identifier
    ) -> Result<(), ScopeError> {
        if self.identifiers.contains_key(&name) {
            return Err(ScopeError::new(name));
        }

        self.identifiers.insert(name, id);

        Ok(())
    }

    pub fn extend(
        &mut self,
        iter: impl IntoIterator<Item=(String, Identifier)>
    ) -> Result<(), ScopeError> {
        for item in iter {
            self.put(item.0, item.1)?;
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Identifier> {
        let maybe_id = self.identifiers.get(name);
        if maybe_id.is_none() && self.outer_scope.is_some() {
            return self.outer_scope.as_ref().unwrap().get(name);
        }

        maybe_id
    }
}

impl<'a> IntoIterator for &'a Scope {
    type Item = (&'a String, &'a Identifier);

    type IntoIter = std::collections::hash_map::Iter<'a, String, Identifier>;

    fn into_iter(self) -> Self::IntoIter {
        self.identifiers.iter()
    }
}

#[derive(Debug)]
pub struct ScopeError {
    id: String
}

impl ScopeError {
    pub fn new(id: String) -> Self {
        Self {
            id: id
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Error for ScopeError {}

impl Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\"{}\" is already present in the scope", self.id)
    }
}
