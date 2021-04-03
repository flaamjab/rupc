use std::{collections::HashSet, io::Write, iter::FromIterator};

use crate::{error::{
        CompilationError,
        CompilationErrorKind,
        Errors
    }, semantics::{
        Enumeration,
        Identifier,
        Fields,
        Identifiers,
        Scope,
        Type,
        Types,
        boolean
    }, tokenization::{
        Token,
        Operator,
        Punctuation,
        Keyword,
        Relation,
        TokenStream,
        Buffer
    }, translation::Wasm};

type ParseResult = Result<(), CompilationError>;

pub struct Code<T: Buffer> {
    token_stream: TokenStream<T>,
    lookahead: Token,
    scope: Box<Scope>,
    errors: Errors,
    wasm: Wasm,
}

impl<T: Buffer> Code<T> {
    const CONTINUE: &'static str = "continue";
    const END: &'static str = "end";
    const R0: &'static str = "r0";

    pub fn new(
        token_stream: TokenStream<T>,
        output: Box<dyn Write>
    ) -> Code<T> {
        Code {
            token_stream: token_stream,
            lookahead: Token::EOF,
            scope: Box::new(Scope::default()),
            errors: Errors::new(),
            wasm: Wasm::new(output),
        }
    }

    /// Compiles the code, producing an executable.
    pub fn compile(mut self) -> Result<Errors, CompilationError> {
        self.token_stream.next().and_then(|token| {
            self.lookahead = token;
            Ok(())
        })?;

        self.program()?;

        Ok(self.errors)
    }

    /// Checks the code for correctness.
    pub fn check(mut self) -> Result<Errors, CompilationError> {
        self.wasm.silence();
        self.compile()
    }

    // <program> ::= program <identifier> ; <block>
    fn program(&mut self) -> ParseResult {
        self.wasm.mod_start();

        if self.lookahead == Token::EOF {
            println!("Input file empty, exiting.");
            return Ok(());
        }

        let procedures = self.scope.into_iter()
            .filter(|(_, id)| {
                if let Identifier::Procedure(_) = id {
                    true
                } else {
                    false
                }
            })
            .map(|(name, id)| {
                if let Identifier::Procedure(t) = id {
                    (name, t)
                } else {
                    panic!("The list must contain only procedures");
                }
            });

        for (name, types) in procedures {
            self.wasm.func_import(name, types)
        }
        
        self.consume(Token::K(Keyword::Program)).and_then(|_| {
            self.identifier()?;
            self.wasm.func_start("program", true);
            self.wasm.func_local(Self::R0, &Type::Integer);
            self.consume(Token::P(Punctuation::Semicolon))
        }).or_else(|_| {
            self.panic(&[
                Token::K(Keyword::Type),
                Token::K(Keyword::Var)
            ])
        }).unwrap_or_default();
        
        self.scope = Scope::empty_with_outer(self.scope.to_owned());
        self.block().or_else(|_| {
            self.panic(&[Token::P(Punctuation::Dot)])
        })?;

        self.consume(Token::P(Punctuation::Dot)).or_else(|_| {
            self.panic(&[Token::EOF])
        }).expect("EOF not found in the stream");

        self.scope = self.scope.clone().collapse().unwrap();

        self.wasm.func_end();
        self.wasm.mod_end();

        Ok(())
    }

    // <block> ::=
        // <type definition part>
        // <variable declaration part> 
        // <statement part>
    fn block(&mut self) -> ParseResult {
        self.type_definitions().or_else(|_| {
            self.panic(&[
                Token::K(Keyword::Var),
                Token::K(Keyword::Begin),
            ])
        })?;

        if let Token::K(Keyword::Var) = self.lookahead {
            self.variable_declarations().or_else(|_| {
                self.panic(&[
                    Token::K(Keyword::Begin),
                    Token::P(Punctuation::Semicolon)
                ])
            })?;
        }

        self.statements()?;

        Ok(())
    }

    // <type definition part> ::=
        // <empty>
        // | type <type definition> {;<type definition>};
    fn type_definitions(&mut self) -> ParseResult {
        if self.lookahead != Token::K(Keyword::Type) {
            return Ok(());
        }
        
        self.consume(Token::K(Keyword::Type))?;
        self.type_definition()?;
        loop {
            if self.lookahead == Token::P(Punctuation::Semicolon) {
                self.proceed()?;
                if !matches!(self.lookahead, Token::Id(_)) {
                    break;
                }
                self.type_definition()?;
            } else {
                break;
            }
        }

        self.consume(Token::P(Punctuation::Semicolon))?;
        
        Ok(())
    }

    // <type definition> ::= <identifier> = <type>
    fn type_definition(&mut self) -> ParseResult {
        let id = self.identifier()?;
        self.consume(Token::R(Relation::Eq))?;
        let t = self.type_()?;

        if let Err(e) = self.scope.put(id, Identifier::Type(t)) {
            self.redefined_identifier(e.id());
        }

        Ok(())
    }

    // <variable declaration part> ::=
        // <empty>
        // | var <variable declaration> {; <variable declaration>} ;
    fn variable_declarations(&mut self) -> ParseResult {
        if self.lookahead != Token::K(Keyword::Var) {
            return Ok(())
        }

        self.proceed()?;
        self.variable_declaration()?;

        loop {
            self.consume(Token::P(Punctuation::Semicolon))?;
            if let Token::Id(_) = self.lookahead {
                self.variable_declaration()?
            } else {
                break
            }
        }

        Ok(())
    }

    // <variable declaration> ::= <identifier> {,<identifier>} : <type>
    fn variable_declaration(&mut self) -> ParseResult {
        let mut names = HashSet::new();
        loop {
            let maybe_name = self.identifier();
            if let Ok(id) = maybe_name {
                if names.contains(&id) {
                    self.redefined_identifier(&id);
                } else {
                    names.insert(id);
                }

                match self.lookahead {
                    Token::P(Punctuation::Comma) => self.proceed()?,
                    Token::P(Punctuation::Colon) => break,
                    _ => ()
                }
            } else {
                self.panic(&[Token::P(Punctuation::Colon)])?
            }
        }

        self.consume(Token::P(Punctuation::Colon))?;

        let t = self.type_()?;

        for name in &names {
            self.wasm.func_local(name, &t.clone());
        }

        let r = self.scope.extend(
            names.drain().map(|name| (
                name.clone(),
                Identifier::Variable(name, t.clone())
            ))
        );

        if let Err(e) = r {
            self.redefined_identifier(e.id());
        }
   
        Ok(())
    }

    // <type> ::= <simple type> | <structured type>
    fn type_(&mut self) -> Result<Type, CompilationError> {
        match self.lookahead {
            Token::K(Keyword::Record) => self.structured_type(),
            _ => self.simple_type()
        }
    }

    // <structured type> ::= <array type> | <record type> | <set type> | <file type>
    fn structured_type(&mut self) -> Result<Type, CompilationError> {
        match self.lookahead {
            Token::K(Keyword::Record) => self.record_type(),
            _ => panic!("Only record structured types are supported")
        }
    }

    // <simple type> ::= <scalar type> | <subrange type> | <type identifier>
    fn simple_type(&mut self) -> Result<Type, CompilationError> {
        match self.lookahead.to_owned() {
            Token::P(Punctuation::Lbracket) => self.scalar_type(),
            Token::Number(_) => self.subrange_type(),
            Token::Id(_) => self.type_identifier(),
            token => Err(self.syntax_error(&format!(
                "expected left bracket, number, or an identifier, found {:?}",
                token
            )))
        }
    }

    // <subrange type> ::= <constant> .. <constant>
    fn subrange_type(&mut self) -> Result<Type, CompilationError> {
        todo!("subrange_type");
    }

    fn type_identifier(&mut self) -> Result<Type, CompilationError> {
        let name = self.identifier()?;

        match self.scope.get(&name) {
            Some(Identifier::Type(t)) => Ok(t.to_owned()),
            Some(_) => {
                self.invalid_identifier("type", &name);
                Ok(Type::Unknown)
            },
            None => {
                self.undeclared_identifier(&name);
                Ok(Type::Unknown)
            }
        }
    }

    // <scalar type> ::= (<identifier> {,<identifier>})
    fn scalar_type(&mut self) -> Result<Type, CompilationError> {
        self.consume(Token::P(Punctuation::Lbracket))?;
        let mut ids = Enumeration::new();
        loop {
            let id = self.identifier()?;
            if ids.contains(&id) {
                self.redefined_identifier(&id);
            } else {
                ids.push_back(id);
            }

            if self.lookahead == Token::P(Punctuation::Comma) {
                self.proceed()?;
            } else {
                self.consume(Token::P(Punctuation::Rbracket))?;
                return Ok(Type::Scalar(ids));
            }
        }
    }

    // <record type> ::= record <field list> end
    fn record_type(&mut self) -> Result<Type, CompilationError> {
        self.consume(Token::K(Keyword::Record))?;
        let fields = self.field_list().or_else(|_| {
            self.panic(&[Token::K(Keyword::End)])?;
            Ok(Fields::new())
        })?;
        self.consume(Token::K(Keyword::End))?;

        Ok(Type::Record(fields))
    }

    // <field list> ::= <fixed part>
    fn field_list(&mut self) -> Result<Fields, CompilationError> {
        let mut table = Fields::new();
        self.fixed_part(&mut table)?;
        Ok(table)
    }

    // <fixed part> ::= <record section> {;<record section>}
    fn fixed_part(
        &mut self, table: &mut Fields
    ) -> ParseResult {
        self.record_section(table)?;

        loop {
            if self.lookahead == Token::P(Punctuation::Semicolon) {
                self.proceed()?;
                self.record_section(table)?;
            } else {
                break;
            }
        }

        Ok(())
    }

    // <record section> ::=
        // <field identifier> {, <field identifier>} : <type>
        // | <empty>
    fn record_section(
        &mut self, table: &mut Fields
    ) -> ParseResult {

        if !matches!(self.lookahead, Token::Id(_)) {
            return Ok(())
        }

        let mut ids = HashSet::new();
        loop {
            let id = self.identifier().or_else(|_| {
                self.panic(&[Token::P(Punctuation::Colon)])?;
                Ok(String::new())
            })?;
            
            if ids.contains(&id) {
                self.redefined_identifier(&id);
            } else {
                ids.insert(id);
            }

            if self.lookahead == Token::P(Punctuation::Comma) {
                self.proceed()?;
            } else {
                break;
            }
        }
        
        
        self.consume(Token::P(Punctuation::Colon))?;
        
        let t = self.type_()?;

        table.extend(ids.drain().map(|id| (id, t.to_owned())));

        Ok(())
    }

    // <statement part> ::= <compound statement>
    fn statements(&mut self) -> ParseResult {
        self.compound_statement()
    }

    // <compound statement> ::= begin <statement> {; <statement> } end;
    fn compound_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::Begin))?;
        self.statement()?;
        loop {
            if self.lookahead == Token::P(Punctuation::Semicolon) {
                self.proceed()?;

                if self.lookahead == Token::K(Keyword::End) {
                    break;
                }

                self.statement()?;
            } else {
                break;
            }
        }

        self.consume(Token::K(Keyword::End))?;

        Ok(())
    }

    // <statement> ::= <simple statement> | <structured statement>
    fn statement(&mut self) -> ParseResult {
        match self.lookahead.clone() {
            Token::P(Punctuation::Semicolon) => Ok(()),
            Token::K(Keyword::End) => Ok(()),
            Token::K(_) => self.structured_statement(),
            Token::Id(_) => self.simple_statement(),
            t => Err(self.syntax_error(&format!(
                "a statement cannot start with {:?}",
                t
            )))
        }
    }

    // <simple statement> ::= <assignment statement> | <empty statement>
    fn simple_statement(&mut self) -> ParseResult {
        if let Token::Id(name) = self.lookahead.clone() {
            match self.scope.get(&name) {
                Some(id) => {
                    let id = id.clone();
                    match id {
                        Identifier::Variable(_, _) =>
                            self.assignment_statement(),
                        Identifier::Procedure(types) =>
                            self.procedure_statement(&name, &types),
                        _ => Err(self.semantic_error("illegal statement"))
                    }
                }
                _ => Err(self.undeclared_identifier(&name)),
            }
        } else {
            panic!("ID token was lost");
        }
    }

    // <assignment statement> ::= <variable> := <expression>
    fn assignment_statement(&mut self) -> ParseResult {
        let (name, variable_type) = self.variable()?;
        self.consume(Token::O(Operator::Assign))?;
        let expression_type = self.expression(&variable_type)?;

        if variable_type != Type::Unknown
            && expression_type != Type::Unknown {

            if variable_type == expression_type {
                self.wasm.local_set(&name)
            } else {
                self.semantic_error("type mismatch in assignment");
            }
        }

        Ok(())
    }

    // <procedure statement> ::=
        // <procedure identifier>
        // | <procedure identifier> (<actual parameter>
            // {, <actual parameter> })
    fn procedure_statement(
        &mut self,
        name: &str,
        types: &Types
    ) -> ParseResult {
        self.identifier()?;
        if types.len() > 0 {
            self.consume(Token::P(Punctuation::Lbracket))?;

            for t in types {
                let t_a = self.expression(t)?;
                if t_a != *t {
                    self.semantic_error(
                        "type mismatch in procedure arguments"
                    );
                }
            }

            self.wasm.call(name);

            self.consume(Token::P(Punctuation::Rbracket))?;
        }

        Ok(())
    }

    // <variable> ::= <identifier> | <identifier> . <field_designator>
    fn variable(
        &mut self
    ) -> Result<(String, Type), CompilationError> {
        
        let name = self.identifier()?;
        let t = match self.scope.get(&name) {
            Some(Identifier::Variable(_, t)) => Ok(t),
            Some(_) => Err(self.invalid_identifier("variable", &name)),
            None => Err(self.undeclared_identifier(&name))
        }?.clone();

        if let Token::P(Punctuation::Dot) = self.lookahead {
            self.proceed()?;
            if let Type::Record(fs) = t {
                let t = self.field_designator(&fs)?;
                Ok((name, t))
            } else {
                self.semantic_error(&format!(
                    "attempt to access a field of a \
                    non-record variable \"{}\"",
                    name,
                ));
                let t = self.field_designator(&Fields::new())?;
                Ok((name, t))
            }
        } else {
            Ok((name, t))
        }
    }

    // <field_designator> ::= 
        // <field_identifier>
        // | <field_identifier> . <field_designator>
    fn field_designator(
        &mut self,
        subscope: &Fields
    ) -> Result<Type, CompilationError> {
        let t = self.field_identifier(subscope)?;
        
        if let Token::P(Punctuation::Dot) = self.lookahead {
            self.proceed()?;
            if let Type::Record(fs) = t {
                self.field_designator(&fs)
            } else {
                self.semantic_error(
                    "attempt to access a field of a non-record field",
                );
                self.field_designator(&Fields::new())
            }
        } else {
            Ok(t)
        }
    }

    // <field_identifier> ::= <identifier>
    fn field_identifier(
        &mut self,
        subscope: &Fields
    ) -> Result<Type, CompilationError> {
        let name = self.identifier()?;
        if subscope.is_empty() {
            return Ok(Type::Unknown);
        }

        if let Some(t) = subscope.get(&name) {
            Ok(t.clone())
        } else {
            self.semantic_error(&format!("undefined field {}", name));
            Ok(Type::Unknown)
        }
    }

    // <structured statement> ::=
        // <compound statement>
        // | <conditional statement>
        // | <loop statement>
        // | <with statement>
    fn structured_statement(&mut self) -> ParseResult {
        match self.lookahead {
            Token::K(Keyword::If) => self.conditional_statement(),
            Token::K(Keyword::For)
            | Token::K(Keyword::While)
            | Token::K(Keyword::Repeat) => self.loop_statement(),
            Token::K(Keyword::Begin) => self.compound_statement(),
            Token::K(Keyword::With) => self.with_statement(),
            Token::K(_) => {
                Err(self.syntax_error(
                    &format!(
                        "keyword {:?} cannot start a statement",
                        self.lookahead
                    )
                ))
            },
            _ => panic!(
                "Keyword token that starts a \
                structured statement was lost"
            )
        }
    }


    // <conditional statement> ::= <if statement>
    fn conditional_statement(&mut self) -> ParseResult {
        self.if_statement()
    }

    // <if statement> ::=
        // if <expression> then <statement>
        // | if <expression> then <statement> else <statement>
    fn if_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::If))?;
        
        self.expression(&boolean())?;
        self.wasm.if_start();

        self.consume(Token::K(Keyword::Then))?;

        self.wasm.then_start();
        self.statement()?;
        self.wasm.then_end();

        if self.lookahead == Token::K(Keyword::Else) {
            self.proceed()?;

            self.wasm.else_start();
            self.statement()?;
            self.wasm.else_end();
        }

        self.wasm.if_end();

        Ok(())
    }
    
    // <loop statement> ::=
        // <while statement>
        // | <repeat statemant>
        // | <for statement>
    fn loop_statement(&mut self) -> ParseResult {
        match self.lookahead {
            Token::K(Keyword::While) => self.while_statement(),
            Token::K(Keyword::Repeat) => self.repeat_statement(),
            Token::K(Keyword::For) => self.for_statement(),
            _ => panic!("Keyword token that opens a loop was lost")
        }
    }

    // <while statement> ::= while <expression> do <statement>
    fn while_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::While))?;

        self.wasm.loop_start(Self::CONTINUE, Self::END);
        self.wasm.constant("1", &Type::Integer);
        let t = self.expression(&boolean()).or_else(|_| {
            self.panic(&[Token::K(Keyword::Do)])?;
            Ok(Type::Unknown)
        })?;
        self.wasm.op(&Operator::Minus, &Type::Integer);

        if t == boolean() {
            self.wasm.br_if(Self::END);
        } else if t != Type::Unknown {
            self.semantic_error(
                "the condition in a while statement must have boolean type"
            );
        }

        self.consume(Token::K(Keyword::Do))?;
        self.statement()?;

        self.wasm.br(Self::CONTINUE);
        self.wasm.loop_end();

        Ok(())
    }

    // <repeat statement> ::= repeat <statement> {; <statement>} until <expression>
    fn repeat_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::Repeat))?;
        self.wasm.loop_start(Self::CONTINUE, Self::END);

        self.statement()?;
        loop {
            if self.lookahead == Token::P(Punctuation::Semicolon) {
                self.proceed()?;
                self.statement()?;
            } else {
                break;
            }
        }

        self.consume(Token::K(Keyword::Until))?;
        let t = self.expression(&boolean())?;
        if t == boolean() {
            self.wasm.br_if(Self::END);
            self.wasm.br(Self::CONTINUE);
        } else if t != Type::Unknown {
            self.semantic_error("until expression must have boolean type");
        }

        self.wasm.loop_end();

        Ok(())
    }
    
    // <for statement> ::= for <control variable> := <for list> do <statement>
    fn for_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::For))?;
        self.wasm.local_get(Self::R0);

        let (n, t) = self.control_variable().or_else(|_| {
            self.panic(&[Token::O(Operator::Assign)])?;
            Ok(("".to_string(), Type::Unknown))
        })?;

        if t != Type::Unknown && t != Type::Integer {
            self.semantic_error(
                "the for-loop control variable must have integer type"
            );
        }

        self.consume(Token::O(Operator::Assign))?;

        let direction = self.for_list(&n)
            .or_else(|_| {
                self.panic(&[Token::K(Keyword::Do)])?;
                Ok(Token::Unknown)
            })?;

        self.wasm.loop_start(Self::CONTINUE, Self::END);
        self.wasm.local_get(Self::R0);
        self.wasm.local_get(&n);
        self.wasm.relop(&Relation::Eq, &Type::Integer);
        self.wasm.br_if(Self::END);

        self.consume(Token::K(Keyword::Do))?;
        self.statement()?;

        self.wasm.constant(
            match direction {
                Token::K(Keyword::To) => "1",
                Token::K(Keyword::Downto) => "-1",
                Token::Unknown => "",
                _ => panic!("Unexpected direction token")
            },
            &Type::Integer
        );
        self.wasm.local_get(&n);
        self.wasm.op(&Operator::Plus, &Type::Integer);
        self.wasm.local_set(&n);

        self.wasm.br(Self::CONTINUE);

        self.wasm.loop_end();

        self.wasm.local_set(Self::R0);

        Ok(())
    }

    // <control variable> ::= <identifier>
    fn control_variable(&mut self) -> Result<(String, Type), CompilationError> {
        let name = self.identifier()?;
        match self.scope.get(&name) {
            Some(Identifier::Variable(n, t)) => Ok((n.clone(), t.clone())),
            Some(_) => Err(self.invalid_identifier("variable", &name)),
            None => Err(self.undeclared_identifier(&name))
        }
    }

    // <for list> ::=
        // <initial value> to <final value>
        // | <initial value> downto <final value>
    fn for_list(&mut self, control_var_name: &str) -> Result<Token, CompilationError> {
        self.initial_value()?;
        self.wasm.local_set(&control_var_name);

        let direction = self.consume_any(&[
            Token::K(Keyword::To),
            Token::K(Keyword::Downto)
        ])?;

        self.final_value()?;
        self.wasm.local_set(Self::R0);

        Ok(direction)
    }

    // <initial value> ::= <expression>
    fn initial_value(&mut self) -> Result<Type, CompilationError> {
        let t = self.expression(&Type::Integer)?;
        if t != Type::Integer {
            self.semantic_error(
                "the initial value in a for loop must have integer type"
            );
            Ok(Type::Unknown)
        } else {
            Ok(t)
        }
    }

    // <final value> ::= <expression>
    fn final_value(&mut self) -> Result<Type, CompilationError> {
        let t = self.expression(&Type::Integer)?;
        if t != Type::Integer {
            self.semantic_error(
                "the final value in a for loop must have integer type"
            );
            Ok(Type::Unknown)
        } else {
            Ok(t)
        }
    }

    // <with statement> ::= with <record variable list> do <statement>
    fn with_statement(&mut self) -> ParseResult {
        self.consume(Token::K(Keyword::With))?;
        let ids = self.record_variables()?;
        self.scope = Scope::with_outer(self.scope.clone(), ids);
        self.consume(Token::K(Keyword::Do))?;
        self.statement()?;

        Ok(())
    }

    // <record variable list> ::= <record variable> {, <record variable>}
    fn record_variables(&mut self) -> Result<Identifiers, CompilationError> {
        let mut table = Fields::new();
        loop {
            let (_, t) = self.variable()?;
            if let Type::Record(fs) = t {
                table.extend(fs)
            } else {
                self.semantic_error("expected a variable of record type");
            }

            if let Token::P(Punctuation::Comma) = self.lookahead {
                self.proceed()?;
            } else {
                break
            }
        }

        let ids = table.drain().map(
            |(k, v)| (k.clone(), Identifier::Variable(k, v))
        ).collect();

        Ok(ids)
    }

    // <expression> ::= 
        // <simple expression> 
        // | <simple expression> <relational operator> <simple expression>
    fn expression(
        &mut self,
        expected_type: &Type
    ) -> Result<Type, CompilationError> {
        let type_a = self.simple_expression(expected_type)?;
        let mut type_r = type_a.clone();

        if let Token::R(op) = self.lookahead {
            self.proceed()?;
            let type_b = self.simple_expression(expected_type)?;

            if type_a == type_b {
                self.wasm.relop(&op, &type_a);
                type_r = boolean();
            } else {
                self.semantic_error(
                    "values of different types cannot be compared"
                );
                type_r = Type::Unknown;
            }
        }

        Ok(type_r)
    }

    // <simple expression> ::=	<sign> <term> { <adding operator> <term> }
    fn simple_expression(
        &mut self,
        expected_type: &Type
    ) -> Result<Type, CompilationError> {
        let mut negative = false;
        if let Token::O(op) = self.lookahead {
            match op {
                Operator::Plus => negative = false,
                Operator::Minus => negative = true,
                _ => return Err(self.syntax_error("expected plus or minus"))
            }
            self.proceed()?;
        }

        if negative {
            self.wasm.constant("0", &Type::Unknown);
        }

        let mut type_ = self.term(expected_type)?;

        if negative {
            self.wasm.fill_nearest_unknown(&type_);
            self.wasm.op(&Operator::Minus, &type_);
        }

        loop {
            if let Token::O(op) = self.lookahead {
                if op.is_adding() {
                    self.proceed()?;
                    let next_type = self.term(expected_type)?;
                    
                    if next_type != type_ {
                        type_ = Type::Unknown;
                    }

                    self.wasm.op(&op, &type_);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        
        Ok(type_)
    }

    // <term> ::= <factor> { <multiplying operator> <factor> }
    fn term(
        &mut self,
        expected_type: &Type
    ) -> Result<Type, CompilationError> {
        let mut type_ = self.factor(expected_type)?;

        loop {
            if let Token::O(op) = self.lookahead {
                if op.is_multiplying() {
                    self.proceed()?;
                    let next_type = self.factor(expected_type)?;

                    if type_ != next_type {
                        type_ = Type::Unknown;
                    }

                    self.wasm.op(&op, &type_);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        
        Ok(type_)
    }

    // <factor> ::=
        // <variable>
        // | <constant>
        // | ( <expression> )
        // | not <factor>
    fn factor(
        &mut self,
        expected_type: &Type
    ) -> Result<Type, CompilationError> {
        match self.lookahead.clone() {
            Token::Id(name) => {
                let mut type_ = Type::Unknown;
                if let Type::Scalar(vs) = expected_type {
                    if let Some(p) = vs.iter().position(|n| n == &name) {
                        type_ = expected_type.to_owned();
                        self.wasm.constant(&p.to_string(), &Type::Integer);
                        self.proceed()?;
                    }
                }

                if type_ == Type::Unknown {
                    let (name, t) = self.variable()?;
                    type_ = t;
                    self.wasm.local_get(&name);
                }

                Ok(type_)
            },
            Token::Number(v) => self.number(&v),
            Token::Literal(v) => self.literal(&v),
            Token::O(Operator::Not) => {
                self.proceed()?;
                return self.factor(expected_type)
            },
            Token::P(Punctuation::Lbracket) => {
                self.proceed()?;
                let type_ = self.expression(expected_type)?;
                self.consume(Token::P(Punctuation::Rbracket))?;
                Ok(type_)
            },
            _ => Err(self.syntax_error("illegal expression"))
        }
    }

    fn number(&mut self, value: &str) -> Result<Type, CompilationError> {
        self.proceed()?;
        let type_;
        if value.contains('.') {
            type_ = Type::Real
        } else {
            type_ = Type::Integer
        }

        self.wasm.constant(value, &type_);

        Ok(type_)
    }

    fn literal(&mut self, value: &str) -> Result<Type, CompilationError> {
        self.proceed()?;
        if value.len() == 1 {
            Ok(Type::Char)
        } else {
            unimplemented!(
                "Character literals longer than 1 symbol are not supported"
            );
        }
    }

    fn identifier(&mut self) -> Result<String, CompilationError> {
        let lookahead = self.lookahead.to_owned();
        match lookahead {
            Token::Id(id) => {
                self.proceed()?;
                Ok(id)
            }
            _ => Err(self.syntax_error(
                &format!(
                    "expected identifier, found {:?}",
                    self.lookahead
                )
            ))
        }
    }

    fn consume(&mut self, token: Token) -> ParseResult {
        if self.lookahead == token {
            self.proceed()
        } else {
            Err(self.syntax_error(
                &format!(
                    "expected {:?}, found {:?}",
                    token,
                    self.lookahead
                )
            ))
        }
    }

    fn consume_any(
        &mut self, tokens: &[Token]
    ) -> Result<Token, CompilationError> {

        let search_result = tokens.iter()
            .find(|&t| self.lookahead == *t);
        if search_result.is_some() {
            self.proceed()?;
            Ok(search_result.unwrap().to_owned())
        } else {
            Err(self.syntax_error(
                &format!(
                    "expected {:?}, found {:?}",
                    tokens,
                    self.lookahead
                )
            ))
        }
    }

    fn proceed(&mut self) -> ParseResult {
        self.lookahead = self.token_stream.next()?;
        Ok(())
    }

    fn panic(&mut self, until_tokens: &[Token]) -> ParseResult {
        if self.token_stream.available(until_tokens)? {
            self.proceed_until(until_tokens)?;
        } else {
            return Err(CompilationError::new(
                CompilationErrorKind::SyntaxError,
                self.token_stream.filepath(),
                self.token_stream.prev_pos(),
                &format!(
                    "failed to recover, none of the \
                    {:?} tokens are present in the stream",
                    until_tokens
                )
            ))
        }

        Ok(())
    }

    fn proceed_until(&mut self, tokens: &[Token]) -> ParseResult {
        let token_set: HashSet<Token> = HashSet::from_iter(tokens.iter().cloned());
        let mut token = self.token_stream.next()?;
        while !token_set.contains(&token) && token != Token::EOF {
            token = self.token_stream.next()?;
        }

        self.lookahead = token;

        Ok(())
    }

    fn invalid_identifier(
        &mut self, expected_kind: &str, name: &str
    ) -> CompilationError {
        self.semantic_error(
            &format!(
                "invalid usage of {}, expected {} identifier",
                name, expected_kind
            )
        )
    }

    fn undeclared_identifier(&mut self, name: &str) -> CompilationError {
        self.scope.put(name.to_string(), Identifier::Unknown).unwrap();
        self.semantic_error(&format!("identifier not found \"{}\"", name))
    }

    fn redefined_identifier(&mut self, name: &str) -> CompilationError {
        self.semantic_error(&format!(
            "duplicate identifier \"{}\"", name
        ))
    }

    fn semantic_error(&mut self, msg: &str) -> CompilationError {
        self.error(CompilationErrorKind::SemanticError, msg)
    }

    fn syntax_error(&mut self, msg: &str) -> CompilationError {
        self.error(CompilationErrorKind::SyntaxError, msg)
    }

    fn error(
        &mut self,
        kind: CompilationErrorKind,
        message: &str
    ) -> CompilationError {
        
        let err = CompilationError::new(
            kind,
            self.token_stream.filepath(),
            self.token_stream.prev_pos(),
            message
        );

        self.wasm.silence();
        self.errors.push(err.clone());

        err
    }

    fn debug(&self, msg: &str) {
        let pos = self.token_stream.pos();
        println!(
            "{}:{}:{}:{:?} => {}",
            self.token_stream.filepath().as_ref().unwrap_or(&"~".to_string()),
            pos.line, pos.col,
            self.lookahead, msg
        );
    }
}

impl Operator {
    fn is_adding(&self) -> bool {
        match self {
            Operator::Plus => true,
            Operator::Minus => true,
            Operator::Or => true,
            _ => false
        }
    }

    fn is_multiplying(&self) -> bool {
        match self {
            Operator::Multiply => true,
            Operator::Divide => true,
            Operator::IntegerDivide => true,
            Operator::And => true,
            _ => false,
        }
    }

    fn is_sign(&self) -> bool {
        match self {
            Operator::Plus => true,
            Operator::Minus => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod code_tests {
    use std::io::stdout;

    use super::*;
    use crate::tokenization::SimpleBuffer;

    fn code(input: &str) -> Code<impl Buffer> {
        let b = SimpleBuffer::new(input.as_bytes(), None);
        let ts = TokenStream::new(b);
        Code::new(ts, Box::new(stdout()))
    }

    /******************************************/
    /*                                        */
    /*        Syntax analysis tests           */
    /*                                        */
    /******************************************/

    #[test]
    fn test_check_empty_program() {
        let input =
            " program Name;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_variables_block() {
        let input =
            " program Name;
              var
                a: Integer;
              begin
              end.
            ";

        let c = code(input);
        let errs = c.check().unwrap();
        assert_eq!(errs.count(), 0);
    }

    #[test]
    fn test_check_missing_semicolon_after_program() {
        let input = 
            " program Name
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_missing_semicolon_in_type_definitions() {
        let input = 
            " program Name;
              type
                a = integer
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_missing_semicolon_in_var_definitions() {
        let input = 
            " program Name;
              var
                a: integer
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_stray_end() {
        let input = 
            " program Name;
              begin
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_record_in_variable_block() {
        let input = 
            " program Name;
              var
                a: record
                  a: Integer;
                end;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_for_loop_correct() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix := 0 to 10 do begin
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_for_loop_missing_direction() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix := 0 10 do begin
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_for_loop_missing_do() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix := 0 to 10
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_for_loop_missing_final() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix := 0 to do
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_for_loop_missing_initial() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix := to 10 do begin
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_for_loop_missing_assignment() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for ix 0 to 10 do
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_for_loop_missing_control_variable() {
        let input = 
        " program Name;
          var
            ix: integer;
          begin
            for := 0 to 10 do begin
              writeln_int(ix)
            end
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_with_statement_one_record() {
        let input = 
            " program Name;
              var
                a: record
                  f: Integer;
                end;
                b: integer;
              begin
                with a do begin
                  b := 0;
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_with_statement_multiple_records() {
        let input = 
            " program Name;
              var
                a: record
                  f_a: Integer;
                end;
                b: record
                  f_b: Integer;
                end;
                c: record
                  f_c: Integer;
                end;
                d: integer;
              begin
                with a, b, c do begin
                  d := 0;
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_long_correct() {
        let input =
            " program Name;
              type
                t1 = Integer;
                t2 = record
                  d: Integer;
                  f: Boolean;
                end;
              var
                a: record
                  b, d: Integer;
                  c: Boolean;
                end;
                b: Integer;
                c: Char;
                ix: Integer;
              begin
                c := 'a';

                if b = 25 then begin
                    a.b := 1;
                    a.c := false;

                    while a.b > 1 do
                        c := 'b'
                end;

                b := 2 + 5*(2-2) + 2;

                repeat begin
                    c := 'j'
                end until 0 <> 0;

                for ix := 0 to 5 do begin
                    b := b + 1;
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_error_recovery() {
        let input =
            " program Name;
              var
                r: record
                  f:: Integer; { second ':' is unexpected but skipped }
                end;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_empty_file() {
        let input = "";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    /******************************************/
    /*                                        */
    /*        Semantic analysis tests         */
    /*                                        */
    /******************************************/

    #[test]
    fn test_check_var_redefinition_global() {
        let input =
            " program Name;
              var
                a: Integer;
                a: Boolean;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_var_redefinition_line() {
        let input =
            " program Name;
              var
                a, a: Integer;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_type_redefinition() {
        let input =
            " program Name;
              type
                a = Integer;
                a = record end;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_type_outer_redefintion() {
        let input =
            " program Name;
              type
                integer = real;
              begin
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_invalid_field_access() {
        let input =
            " program Name;
              var
                a: Integer;
              begin
                a.b := 0;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_non_existent_field_access() {
        let input =
        " program Name;
          var
            a: record
              a: Integer;
            end;
          begin
            a.b := 0;
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_non_existent_field_field_access() {
        let input =
        " program Name;
          var
            a: record
              a: Integer;
            end;
          begin
            a.b.c := 0;
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 2);
    }

    #[test]
    fn test_check_field_field_access() {
        let input =
        " program Name;
          var
            a: record
              b: record
                c: Integer;
              end;
            end;
          begin
            a.b.c := 0;
          end.
        ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_bad_assignment() {
        let input =
            " program Name;
              var
                a: Integer;
                b: Boolean;
              begin
                a := b;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);       
    }

    #[test]
    fn test_check_deep_assignment() {
        let input =
            " program Name;
              var
                a: record
                  b: record
                    c: Integer;
                  end;
                end;

                b: record
                  c: Integer;
                end;
              begin
                a.b.c := b.c;
                b.c := a.b.c;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);      
    }

    #[test]
    fn test_check_alias_assignment() {
        let input =
            " program Name;
              type
                t_a = integer;
                t_b = integer;
              var
                a: t_a;
                b: t_b;
              begin
                a := b;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);   
    }

    #[test]
    fn test_check_deep_alias_assignment() {
        let input =
            " program Name;
              type
                t_a = integer;
                t_b = t_a;
              var
                a: t_a;
                b: t_b;
              begin
                a := b;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_deep_incorrect_alias_assignment() {
        let input =
            " program Name;
              type
                t_a = integer;
                t_b = t_a;
                t_c = boolean;
              var
                a: t_b;
                b: t_c;
              begin
                a := b;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_boolean_assignment() {
        let input =
            " program Name;
              var
                a: boolean;
              begin
                a := true;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0); 
    }

    #[test]
    fn test_check_scalar_type() {
        let input =
            " program Name;
              var
                a: (Apple, Banana, Grape);
              begin
                a := apple;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_expression() {
        let input =
            " program Name;
              var
                result: integer;
              begin
                result := -2 + 5*10;
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_expression_with_negative_number_in_if() {
        let input =
            " program Name;
              var
                result: integer;
              begin
                if -2 < -4 then
                begin
                    result := -2;
                end else begin
                    result := 0;
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 0);
    }

    #[test]
    fn test_check_with_statement_undefined_field() {
        let input =
            " program Name;
              var
                a: record
                  f: integer
                end;
              begin
                with a do begin
                  f_u := 0
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    #[test]
    fn test_check_with_statement_shadowed_field_leading_to_type_mismatch() {
        let input =
            " program Name;
              var
                a: record
                  f: integer
                end;
                b: record
                  f: real
                end;
              begin
                with a, b do begin
                  f := 0
                end
              end.
            ";

        let c = code(input);
        assert_errors_count(c, 1);
    }

    fn assert_errors_count(code: Code<impl Buffer>, count: usize) {
        let errs = code.check().unwrap();
        println!("{}", errs);
        assert_eq!(count, errs.count()); 
    }
}
