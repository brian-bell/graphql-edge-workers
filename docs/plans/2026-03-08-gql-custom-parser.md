# gql-custom-parser Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers-extended-cc:executing-plans to implement this plan task-by-task.

**Goal:** Build a Cloudflare Worker in Rust that serves a flight-log GraphQL API using a hand-written lexer, parser, and executor, resolving data via HTTP calls to an upstream origin API. Partial GraphQL spec: fields, arguments, variables, aliases.

**Architecture:** Single `POST /graphql` endpoint. The Worker entry point (`lib.rs`) routes requests to a handler. The handler deserializes the GraphQL request JSON, passes the query string to a lexer → parser → AST pipeline, validates the AST against a hardcoded schema definition, then executes it by walking the AST and dispatching HTTP calls to the origin. Response is assembled as a JSON value.

**Tech Stack:** Rust, `worker` 0.7 (with `http` feature), `serde`/`serde_json`, `wrangler` CLI. No GraphQL library.

---

### Task 0: Scaffold the crate and verify it compiles

**Files:**
- Modify: `Cargo.toml` (workspace root — add new member)
- Create: `workers/gql-custom-parser/Cargo.toml`
- Create: `workers/gql-custom-parser/src/lib.rs`
- Create: `workers/gql-custom-parser/wrangler.toml`

**Step 1: Add workspace member**

Update root `Cargo.toml`:

```toml
[workspace]
members = ["workers/gql-async-graphql", "workers/gql-custom-parser"]
resolver = "2"
```

**Step 2: Create worker Cargo.toml**

```toml
[package]
name = "gql-custom-parser"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
worker = { version = "0.7", features = ["http"] }
worker-macros = { version = "0.7", features = ["http"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen-futures = "0.4"

[profile.release]
opt-level = "s"
lto = true
```

**Step 3: Create minimal lib.rs**

```rust
use worker::*;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    _env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let response = http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"ok": true}"#.to_string())
        .unwrap();
    Ok(response)
}
```

**Step 4: Create wrangler.toml**

```toml
name = "gql-custom-parser"
main = "build/worker/shim.mjs"
compatibility_date = "2026-03-08"

[build]
command = "cargo install -q worker-build && worker-build --release"

[vars]
ORIGIN_BASE_URL = "http://localhost:8080"
```

**Step 5: Verify it compiles**

Run: `cd workers/gql-custom-parser && npx wrangler dev`
Expected: Worker starts, `curl http://localhost:8787/` returns `{"ok": true}`

**Step 6: Commit**

```bash
git add Cargo.toml workers/gql-custom-parser/
git commit -m "feat: scaffold gql-custom-parser worker crate"
```

---

### Task 1: Add health endpoint and request routing

**Files:**
- Modify: `workers/gql-custom-parser/src/lib.rs`
- Create: `workers/gql-custom-parser/src/handler.rs`

**Step 1: Implement routing in lib.rs**

```rust
use worker::*;

mod handler;

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<http::Response<String>> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    match (method, path.as_str()) {
        (http::Method::GET, "/health") => handler::health(),
        (http::Method::POST, "/graphql") => handler::graphql(req, env).await,
        _ => Ok(http::Response::builder()
            .status(404)
            .body("Not Found".to_string())
            .unwrap()),
    }
}
```

**Step 2: Create handler.rs (stub)**

```rust
use worker::*;

pub fn health() -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok"}"#.to_string())
        .unwrap())
}

pub async fn graphql(
    _req: HttpRequest,
    _env: Env,
) -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"data":null}"#.to_string())
        .unwrap())
}
```

**Step 3: Verify**

Run: `cd workers/gql-custom-parser && npx wrangler dev`
- `curl http://localhost:8787/health` → `{"status":"ok"}`

**Step 4: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: add health endpoint and request routing"
```

---

### Task 2: Implement the lexer

**Files:**
- Create: `workers/gql-custom-parser/src/parser/mod.rs`
- Create: `workers/gql-custom-parser/src/parser/lexer.rs`
- Modify: `workers/gql-custom-parser/src/lib.rs`

**Step 1: Write failing tests for the lexer**

Create `lexer.rs` with tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_simple_query() {
        let tokens = lex("{ flights { id } }").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::BraceOpen,
                Token::Name("flights".to_string()),
                Token::BraceOpen,
                Token::Name("id".to_string()),
                Token::BraceClose,
                Token::BraceClose,
            ]
        );
    }

    #[test]
    fn test_lex_with_arguments() {
        let tokens = lex(r#"{ flight(id: "123") { id } }"#).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::BraceOpen,
                Token::Name("flight".to_string()),
                Token::ParenOpen,
                Token::Name("id".to_string()),
                Token::Colon,
                Token::StringLiteral("123".to_string()),
                Token::ParenClose,
                Token::BraceOpen,
                Token::Name("id".to_string()),
                Token::BraceClose,
                Token::BraceClose,
            ]
        );
    }

    #[test]
    fn test_lex_integer() {
        let tokens = lex("{ flights(limit: 10) { id } }").unwrap();
        assert!(tokens.contains(&Token::IntLiteral(10)));
    }

    #[test]
    fn test_lex_float() {
        let tokens = lex("{ flights(lat: 37.5) { id } }").unwrap();
        assert!(tokens.contains(&Token::FloatLiteral(37.5)));
    }

    #[test]
    fn test_lex_variable() {
        let tokens = lex("query($id: ID!) { flight(id: $id) { id } }").unwrap();
        assert!(tokens.contains(&Token::Dollar));
        assert!(tokens.contains(&Token::Bang));
    }

    #[test]
    fn test_lex_alias() {
        let tokens = lex("{ myFlight: flight(id: \"1\") { id } }").unwrap();
        assert!(tokens.contains(&Token::Colon));
    }

    #[test]
    fn test_lex_null() {
        let tokens = lex("{ flight(id: null) { id } }").unwrap();
        assert!(tokens.contains(&Token::Null));
    }

    #[test]
    fn test_lex_boolean() {
        let tokens = lex("{ flights(active: true) { id } }").unwrap();
        assert!(tokens.contains(&Token::BoolLiteral(true)));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p gql-custom-parser`
Expected: FAIL — module doesn't exist

**Step 3: Implement the lexer**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Punctuation
    BraceOpen,    // {
    BraceClose,   // }
    ParenOpen,    // (
    ParenClose,   // )
    BracketOpen,  // [
    BracketClose, // ]
    Colon,        // :
    Dollar,       // $
    Bang,         // !
    Comma,        // ,

    // Literals
    Name(String),
    StringLiteral(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    Null,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lex error at position {}: {}", self.position, self.message)
    }
}

pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        match chars[pos] {
            // Skip whitespace and commas (commas are insignificant in GraphQL)
            ' ' | '\t' | '\n' | '\r' => {
                pos += 1;
            }
            // Skip comments
            '#' => {
                while pos < chars.len() && chars[pos] != '\n' {
                    pos += 1;
                }
            }
            '{' => { tokens.push(Token::BraceOpen); pos += 1; }
            '}' => { tokens.push(Token::BraceClose); pos += 1; }
            '(' => { tokens.push(Token::ParenOpen); pos += 1; }
            ')' => { tokens.push(Token::ParenClose); pos += 1; }
            '[' => { tokens.push(Token::BracketOpen); pos += 1; }
            ']' => { tokens.push(Token::BracketClose); pos += 1; }
            ':' => { tokens.push(Token::Colon); pos += 1; }
            '$' => { tokens.push(Token::Dollar); pos += 1; }
            '!' => { tokens.push(Token::Bang); pos += 1; }
            ',' => { pos += 1; } // commas are insignificant in GraphQL
            '"' => {
                pos += 1; // skip opening quote
                let start = pos;
                while pos < chars.len() && chars[pos] != '"' {
                    if chars[pos] == '\\' {
                        pos += 1; // skip escaped char
                    }
                    pos += 1;
                }
                if pos >= chars.len() {
                    return Err(LexError {
                        message: "Unterminated string".to_string(),
                        position: start - 1,
                    });
                }
                let value: String = chars[start..pos].iter().collect();
                tokens.push(Token::StringLiteral(value));
                pos += 1; // skip closing quote
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = pos;
                while pos < chars.len() && (chars[pos].is_ascii_alphanumeric() || chars[pos] == '_') {
                    pos += 1;
                }
                let name: String = chars[start..pos].iter().collect();
                match name.as_str() {
                    "true" => tokens.push(Token::BoolLiteral(true)),
                    "false" => tokens.push(Token::BoolLiteral(false)),
                    "null" => tokens.push(Token::Null),
                    _ => tokens.push(Token::Name(name)),
                }
            }
            c if c.is_ascii_digit() || c == '-' => {
                let start = pos;
                if c == '-' {
                    pos += 1;
                }
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                let is_float = pos < chars.len() && chars[pos] == '.';
                if is_float {
                    pos += 1; // skip dot
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                    let num_str: String = chars[start..pos].iter().collect();
                    let value: f64 = num_str.parse().map_err(|_| LexError {
                        message: format!("Invalid float: {num_str}"),
                        position: start,
                    })?;
                    tokens.push(Token::FloatLiteral(value));
                } else {
                    let num_str: String = chars[start..pos].iter().collect();
                    let value: i64 = num_str.parse().map_err(|_| LexError {
                        message: format!("Invalid integer: {num_str}"),
                        position: start,
                    })?;
                    tokens.push(Token::IntLiteral(value));
                }
            }
            c => {
                return Err(LexError {
                    message: format!("Unexpected character: '{c}'"),
                    position: pos,
                });
            }
        }
    }

    Ok(tokens)
}
```

**Step 4: Create parser/mod.rs**

```rust
pub mod lexer;
pub mod ast;
pub mod validate;
```

**Step 5: Create placeholder ast.rs and validate.rs**

`ast.rs`:
```rust
// AST types — implemented in Task 3
```

`validate.rs`:
```rust
// Validation — implemented in Task 5
```

**Step 6: Add `mod parser;` to lib.rs**

**Step 7: Run tests to verify they pass**

Run: `cargo test -p gql-custom-parser`
Expected: All 8 lexer tests PASS

**Step 8: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: implement GraphQL lexer with full token support"
```

---

### Task 3: Define AST types

**Files:**
- Modify: `workers/gql-custom-parser/src/parser/ast.rs`

**Step 1: Define AST types**

No tests for pure data types — they're validated by usage in the parser tests (Task 4).

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    Query,
    Mutation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub operation: Operation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    pub operation_type: OperationType,
    pub name: Option<String>,
    pub variable_definitions: Vec<VariableDefinition>,
    pub selection_set: SelectionSet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDefinition {
    pub name: String,
    pub type_name: String,
    pub non_null: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionSet {
    pub selections: Vec<Selection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub alias: Option<String>,
    pub name: String,
    pub arguments: Vec<Argument>,
    pub selection_set: Option<SelectionSet>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Variable(String),
    List(Vec<Value>),
}
```

**Step 2: Verify it compiles**

Run: `cargo test -p gql-custom-parser`
Expected: Previous lexer tests still pass, no new failures

**Step 3: Commit**

```bash
git add workers/gql-custom-parser/src/parser/ast.rs
git commit -m "feat: define GraphQL AST types"
```

---

### Task 4: Implement the parser

**Files:**
- Create: `workers/gql-custom-parser/src/parser/parse.rs`
- Modify: `workers/gql-custom-parser/src/parser/mod.rs`

**Step 1: Write failing tests for the parser**

Add to `parse.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_parse_simple_query() {
        let doc = parse("{ flights { id date } }").unwrap();
        assert_eq!(doc.operation.operation_type, OperationType::Query);
        assert_eq!(doc.operation.selection_set.selections.len(), 1);
        let flights = &doc.operation.selection_set.selections[0];
        assert_eq!(flights.name, "flights");
        let nested = flights.selection_set.as_ref().unwrap();
        assert_eq!(nested.selections.len(), 2);
        assert_eq!(nested.selections[0].name, "id");
        assert_eq!(nested.selections[1].name, "date");
    }

    #[test]
    fn test_parse_query_with_arguments() {
        let doc = parse(r#"{ flight(id: "123") { id date } }"#).unwrap();
        let flight = &doc.operation.selection_set.selections[0];
        assert_eq!(flight.name, "flight");
        assert_eq!(flight.arguments.len(), 1);
        assert_eq!(flight.arguments[0].name, "id");
        assert_eq!(flight.arguments[0].value, Value::String("123".to_string()));
    }

    #[test]
    fn test_parse_query_with_int_argument() {
        let doc = parse("{ flights(limit: 10) { id } }").unwrap();
        let flights = &doc.operation.selection_set.selections[0];
        assert_eq!(flights.arguments[0].value, Value::Int(10));
    }

    #[test]
    fn test_parse_alias() {
        let doc = parse(r#"{ myFlight: flight(id: "1") { id } }"#).unwrap();
        let sel = &doc.operation.selection_set.selections[0];
        assert_eq!(sel.alias, Some("myFlight".to_string()));
        assert_eq!(sel.name, "flight");
    }

    #[test]
    fn test_parse_variables() {
        let doc = parse("query GetFlight($id: ID!) { flight(id: $id) { id } }").unwrap();
        assert_eq!(doc.operation.variable_definitions.len(), 1);
        assert_eq!(doc.operation.variable_definitions[0].name, "id");
        assert_eq!(doc.operation.variable_definitions[0].type_name, "ID");
        assert!(doc.operation.variable_definitions[0].non_null);
        let flight = &doc.operation.selection_set.selections[0];
        assert_eq!(
            flight.arguments[0].value,
            Value::Variable("id".to_string())
        );
    }

    #[test]
    fn test_parse_mutation() {
        let doc = parse(r#"mutation { createFlight(input: {date: "2026-03-08"}) { id } }"#).unwrap();
        assert_eq!(doc.operation.operation_type, OperationType::Mutation);
    }

    #[test]
    fn test_parse_explicit_query_keyword() {
        let doc = parse("query { flights { id } }").unwrap();
        assert_eq!(doc.operation.operation_type, OperationType::Query);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p gql-custom-parser`
Expected: FAIL — `parse` module doesn't exist

**Step 3: Implement the parser**

```rust
use crate::parser::ast::*;
use crate::parser::lexer::{self, Token, LexError};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError { message: e.to_string() }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, ParseError> {
        match self.advance() {
            Some(ref t) if std::mem::discriminant(t) == std::mem::discriminant(expected) => {
                Ok(t.clone())
            }
            Some(t) => Err(ParseError {
                message: format!("Expected {expected:?}, got {t:?}"),
            }),
            None => Err(ParseError {
                message: format!("Expected {expected:?}, got end of input"),
            }),
        }
    }

    fn expect_name(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Some(Token::Name(n)) => Ok(n),
            Some(t) => Err(ParseError {
                message: format!("Expected name, got {t:?}"),
            }),
            None => Err(ParseError {
                message: "Expected name, got end of input".to_string(),
            }),
        }
    }

    fn parse_document(&mut self) -> Result<Document, ParseError> {
        let operation = self.parse_operation()?;
        Ok(Document { operation })
    }

    fn parse_operation(&mut self) -> Result<Operation, ParseError> {
        let (operation_type, name) = match self.peek() {
            Some(Token::BraceOpen) => (OperationType::Query, None),
            Some(Token::Name(n)) if n == "query" => {
                self.advance();
                let name = match self.peek() {
                    Some(Token::Name(_)) => {
                        let n = self.expect_name()?;
                        Some(n)
                    }
                    _ => None,
                };
                (OperationType::Query, name)
            }
            Some(Token::Name(n)) if n == "mutation" => {
                self.advance();
                let name = match self.peek() {
                    Some(Token::Name(_)) => {
                        let n = self.expect_name()?;
                        Some(n)
                    }
                    _ => None,
                };
                (OperationType::Mutation, name)
            }
            Some(t) => {
                return Err(ParseError {
                    message: format!("Expected query or mutation, got {t:?}"),
                })
            }
            None => {
                return Err(ParseError {
                    message: "Empty query".to_string(),
                })
            }
        };

        let variable_definitions = if matches!(self.peek(), Some(Token::ParenOpen)) {
            self.parse_variable_definitions()?
        } else {
            vec![]
        };

        let selection_set = self.parse_selection_set()?;

        Ok(Operation {
            operation_type,
            name,
            variable_definitions,
            selection_set,
        })
    }

    fn parse_variable_definitions(&mut self) -> Result<Vec<VariableDefinition>, ParseError> {
        self.expect(&Token::ParenOpen)?;
        let mut defs = Vec::new();

        while !matches!(self.peek(), Some(Token::ParenClose)) {
            self.expect(&Token::Dollar)?;
            let name = self.expect_name()?;
            self.expect(&Token::Colon)?;
            let type_name = self.expect_name()?;
            let non_null = if matches!(self.peek(), Some(Token::Bang)) {
                self.advance();
                true
            } else {
                false
            };
            defs.push(VariableDefinition {
                name,
                type_name,
                non_null,
            });
        }

        self.expect(&Token::ParenClose)?;
        Ok(defs)
    }

    fn parse_selection_set(&mut self) -> Result<SelectionSet, ParseError> {
        self.expect(&Token::BraceOpen)?;
        let mut selections = Vec::new();

        while !matches!(self.peek(), Some(Token::BraceClose)) {
            selections.push(self.parse_selection()?);
        }

        self.expect(&Token::BraceClose)?;
        Ok(SelectionSet { selections })
    }

    fn parse_selection(&mut self) -> Result<Selection, ParseError> {
        let first_name = self.expect_name()?;

        // Check for alias: if next token is ':', this is alias: fieldName
        let (alias, name) = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance(); // consume ':'
            let field_name = self.expect_name()?;
            (Some(first_name), field_name)
        } else {
            (None, first_name)
        };

        let arguments = if matches!(self.peek(), Some(Token::ParenOpen)) {
            self.parse_arguments()?
        } else {
            vec![]
        };

        let selection_set = if matches!(self.peek(), Some(Token::BraceOpen)) {
            Some(self.parse_selection_set()?)
        } else {
            None
        };

        Ok(Selection {
            alias,
            name,
            arguments,
            selection_set,
        })
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, ParseError> {
        self.expect(&Token::ParenOpen)?;
        let mut args = Vec::new();

        while !matches!(self.peek(), Some(Token::ParenClose)) {
            let name = self.expect_name()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_value()?;
            args.push(Argument { name, value });
        }

        self.expect(&Token::ParenClose)?;
        Ok(args)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        match self.peek().cloned() {
            Some(Token::StringLiteral(_)) => {
                if let Token::StringLiteral(s) = self.advance().unwrap() {
                    Ok(Value::String(s))
                } else {
                    unreachable!()
                }
            }
            Some(Token::IntLiteral(_)) => {
                if let Token::IntLiteral(i) = self.advance().unwrap() {
                    Ok(Value::Int(i))
                } else {
                    unreachable!()
                }
            }
            Some(Token::FloatLiteral(_)) => {
                if let Token::FloatLiteral(f)) = self.advance().unwrap() {
                    Ok(Value::Float(f))
                } else {
                    unreachable!()
                }
            }
            Some(Token::BoolLiteral(_)) => {
                if let Token::BoolLiteral(b) = self.advance().unwrap() {
                    Ok(Value::Bool(b))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Null) => {
                self.advance();
                Ok(Value::Null)
            }
            Some(Token::Dollar) => {
                self.advance();
                let name = self.expect_name()?;
                Ok(Value::Variable(name))
            }
            Some(Token::BraceOpen) => {
                // Object literal (for input types like CreateFlightInput)
                self.parse_object_value()
            }
            Some(Token::BracketOpen) => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.peek(), Some(Token::BracketClose)) {
                    items.push(self.parse_value()?);
                }
                self.expect(&Token::BracketClose)?;
                Ok(Value::List(items))
            }
            Some(t) => Err(ParseError {
                message: format!("Expected value, got {t:?}"),
            }),
            None => Err(ParseError {
                message: "Expected value, got end of input".to_string(),
            }),
        }
    }

    fn parse_object_value(&mut self) -> Result<Value, ParseError> {
        // For input objects like {date: "2026-03-08", notes: "test"}
        // We represent them as Value::Object — but we need to add this variant.
        // For now, skip over the object and store as a list of key-value pairs
        // by reusing our existing types.
        self.expect(&Token::BraceOpen)?;
        let mut pairs = Vec::new();
        while !matches!(self.peek(), Some(Token::BraceClose)) {
            let key = self.expect_name()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_value()?;
            pairs.push((key, value));
        }
        self.expect(&Token::BraceClose)?;
        Ok(Value::Object(pairs))
    }
}

pub fn parse(input: &str) -> Result<Document, ParseError> {
    let tokens = lexer::lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_document()
}
```

**Step 4: Add Object variant to Value enum in ast.rs**

Update the `Value` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Variable(String),
    List(Vec<Value>),
    Object(Vec<(String, Value)>),
}
```

**Step 5: Add `pub mod parse;` to parser/mod.rs**

**Step 6: Run tests to verify they pass**

Run: `cargo test -p gql-custom-parser`
Expected: All parser + lexer tests PASS

**Step 7: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: implement GraphQL parser with support for fields, args, variables, aliases"
```

---

### Task 5: Implement schema definition and validation

**Files:**
- Modify: `workers/gql-custom-parser/src/parser/validate.rs`
- Create: `workers/gql-custom-parser/src/schema.rs`
- Modify: `workers/gql-custom-parser/src/lib.rs`

**Step 1: Write failing tests for validation**

In `validate.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse::parse;
    use crate::schema::flight_schema;

    #[test]
    fn test_valid_query() {
        let doc = parse("{ flights { id date } }").unwrap();
        let schema = flight_schema();
        assert!(validate(&doc, &schema).is_ok());
    }

    #[test]
    fn test_invalid_field() {
        let doc = parse("{ flights { id nonexistent } }").unwrap();
        let schema = flight_schema();
        assert!(validate(&doc, &schema).is_err());
    }

    #[test]
    fn test_invalid_root_field() {
        let doc = parse("{ nonexistent { id } }").unwrap();
        let schema = flight_schema();
        assert!(validate(&doc, &schema).is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p gql-custom-parser`
Expected: FAIL — schema module and validate function don't exist

**Step 3: Implement schema definition (schema.rs)**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FieldType {
    Scalar(String),        // "String", "Int", "Float", "Boolean", "ID"
    Object(String),        // reference to another type name
    List(Box<FieldType>),
    NonNull(Box<FieldType>),
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub arguments: Vec<ArgumentDef>,
}

#[derive(Debug, Clone)]
pub struct ArgumentDef {
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub fields: HashMap<String, FieldDef>,
}

#[derive(Debug, Clone)]
pub struct SchemaDef {
    pub query_type: TypeDef,
    pub mutation_type: Option<TypeDef>,
    pub types: HashMap<String, TypeDef>,
}

pub fn flight_schema() -> SchemaDef {
    let flight_fields: HashMap<String, FieldDef> = [
        ("id", FieldType::NonNull(Box::new(FieldType::Scalar("ID".into())))),
        ("date", FieldType::NonNull(Box::new(FieldType::Scalar("String".into())))),
        ("aircraftTitle", FieldType::Scalar("String".into())),
        ("aircraftRegistration", FieldType::Scalar("String".into())),
        ("departureIcao", FieldType::Scalar("String".into())),
        ("departureName", FieldType::Scalar("String".into())),
        ("departureLat", FieldType::Scalar("Float".into())),
        ("departureLon", FieldType::Scalar("Float".into())),
        ("arrivalIcao", FieldType::Scalar("String".into())),
        ("arrivalName", FieldType::Scalar("String".into())),
        ("arrivalLat", FieldType::Scalar("Float".into())),
        ("arrivalLon", FieldType::Scalar("Float".into())),
        ("distanceNm", FieldType::Scalar("Float".into())),
        ("elapsedSeconds", FieldType::Scalar("Int".into())),
        ("maxAltitudeFt", FieldType::Scalar("Float".into())),
        ("landingVsFpm", FieldType::Scalar("Float".into())),
        ("landingGForce", FieldType::Scalar("Float".into())),
        ("notes", FieldType::Scalar("String".into())),
    ]
    .into_iter()
    .map(|(name, ft)| {
        (
            name.to_string(),
            FieldDef {
                name: name.to_string(),
                field_type: ft,
                arguments: vec![],
            },
        )
    })
    .collect();

    let flight_type = TypeDef {
        name: "Flight".to_string(),
        fields: flight_fields,
    };

    let mut query_fields = HashMap::new();
    query_fields.insert(
        "flight".to_string(),
        FieldDef {
            name: "flight".to_string(),
            field_type: FieldType::Object("Flight".into()),
            arguments: vec![ArgumentDef {
                name: "id".to_string(),
                field_type: FieldType::NonNull(Box::new(FieldType::Scalar("ID".into()))),
            }],
        },
    );
    query_fields.insert(
        "flights".to_string(),
        FieldDef {
            name: "flights".to_string(),
            field_type: FieldType::NonNull(Box::new(FieldType::List(Box::new(
                FieldType::NonNull(Box::new(FieldType::Object("Flight".into()))),
            )))),
            arguments: vec![
                ArgumentDef {
                    name: "limit".to_string(),
                    field_type: FieldType::Scalar("Int".into()),
                },
                ArgumentDef {
                    name: "offset".to_string(),
                    field_type: FieldType::Scalar("Int".into()),
                },
            ],
        },
    );

    let query_type = TypeDef {
        name: "Query".to_string(),
        fields: query_fields,
    };

    let mut mutation_fields = HashMap::new();
    mutation_fields.insert(
        "createFlight".to_string(),
        FieldDef {
            name: "createFlight".to_string(),
            field_type: FieldType::NonNull(Box::new(FieldType::Object("Flight".into()))),
            arguments: vec![ArgumentDef {
                name: "input".to_string(),
                field_type: FieldType::NonNull(Box::new(FieldType::Object(
                    "CreateFlightInput".into(),
                ))),
            }],
        },
    );

    let mutation_type = TypeDef {
        name: "Mutation".to_string(),
        fields: mutation_fields,
    };

    let mut types = HashMap::new();
    types.insert("Flight".to_string(), flight_type);

    SchemaDef {
        query_type,
        mutation_type: Some(mutation_type),
        types,
    }
}
```

**Step 4: Implement validate.rs**

```rust
use crate::parser::ast::*;
use crate::schema::{SchemaDef, TypeDef, FieldType};

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

pub fn validate(doc: &Document, schema: &SchemaDef) -> Result<(), ValidationError> {
    let root_type = match doc.operation.operation_type {
        OperationType::Query => &schema.query_type,
        OperationType::Mutation => {
            schema.mutation_type.as_ref().ok_or(ValidationError {
                message: "Schema does not support mutations".to_string(),
            })?
        }
    };

    validate_selection_set(&doc.operation.selection_set, root_type, schema)
}

fn validate_selection_set(
    set: &SelectionSet,
    parent_type: &TypeDef,
    schema: &SchemaDef,
) -> Result<(), ValidationError> {
    for selection in &set.selections {
        let field_def = parent_type.fields.get(&selection.name).ok_or(ValidationError {
            message: format!(
                "Field '{}' not found on type '{}'",
                selection.name, parent_type.name
            ),
        })?;

        if let Some(ref nested) = selection.selection_set {
            let nested_type_name = resolve_type_name(&field_def.field_type);
            if let Some(type_name) = nested_type_name {
                let nested_type = schema.types.get(type_name).ok_or(ValidationError {
                    message: format!("Type '{type_name}' not found in schema"),
                })?;
                validate_selection_set(nested, nested_type, schema)?;
            }
        }
    }
    Ok(())
}

fn resolve_type_name(ft: &FieldType) -> Option<&str> {
    match ft {
        FieldType::Object(name) => Some(name),
        FieldType::NonNull(inner) | FieldType::List(inner) => resolve_type_name(inner),
        FieldType::Scalar(_) => None,
    }
}
```

**Step 5: Add `mod schema;` to lib.rs**

**Step 6: Run tests to verify they pass**

Run: `cargo test -p gql-custom-parser`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: implement schema definition and query validation"
```

---

### Task 6: Implement the execution engine

**Files:**
- Create: `workers/gql-custom-parser/src/execute.rs`
- Modify: `workers/gql-custom-parser/src/lib.rs`

**Step 1: Write failing tests**

In `execute.rs`, test that the executor correctly maps a parsed query to the expected upstream HTTP calls and assembles a response. Since the executor depends on `worker::Fetch`, we test the response assembly logic separately:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse::parse;
    use crate::schema::flight_schema;

    #[test]
    fn test_resolve_variables() {
        let mut variables = serde_json::Map::new();
        variables.insert("id".to_string(), serde_json::json!("42"));
        let val = resolve_value(&Value::Variable("id".to_string()), &variables);
        assert_eq!(val, serde_json::json!("42"));
    }

    #[test]
    fn test_resolve_literal_values() {
        let variables = serde_json::Map::new();
        assert_eq!(
            resolve_value(&Value::String("hello".into()), &variables),
            serde_json::json!("hello")
        );
        assert_eq!(
            resolve_value(&Value::Int(42), &variables),
            serde_json::json!(42)
        );
        assert_eq!(
            resolve_value(&Value::Float(3.14), &variables),
            serde_json::json!(3.14)
        );
        assert_eq!(
            resolve_value(&Value::Bool(true), &variables),
            serde_json::json!(true)
        );
        assert_eq!(
            resolve_value(&Value::Null, &variables),
            serde_json::Value::Null
        );
    }

    #[test]
    fn test_pick_fields_from_data() {
        let data = serde_json::json!({
            "id": "1",
            "date": "2026-03-08",
            "aircraftTitle": "C172",
            "notes": null,
            "extra_field": "should be excluded"
        });

        let doc = parse("{ flight(id: \"1\") { id date aircraftTitle } }").unwrap();
        let selections = &doc.operation.selection_set.selections[0]
            .selection_set
            .as_ref()
            .unwrap()
            .selections;

        let variables = serde_json::Map::new();
        let result = pick_fields(&data, selections, &variables);
        assert_eq!(
            result,
            serde_json::json!({
                "id": "1",
                "date": "2026-03-08",
                "aircraftTitle": "C172"
            })
        );
    }

    #[test]
    fn test_pick_fields_with_alias() {
        let data = serde_json::json!({
            "id": "1",
            "date": "2026-03-08"
        });

        let doc = parse("{ flight(id: \"1\") { flightId: id date } }").unwrap();
        let selections = &doc.operation.selection_set.selections[0]
            .selection_set
            .as_ref()
            .unwrap()
            .selections;

        let variables = serde_json::Map::new();
        let result = pick_fields(&data, selections, &variables);
        assert_eq!(
            result,
            serde_json::json!({
                "flightId": "1",
                "date": "2026-03-08"
            })
        );
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p gql-custom-parser`
Expected: FAIL — execute module doesn't exist

**Step 3: Implement execute.rs**

```rust
use crate::parser::ast::*;
use crate::schema::SchemaDef;
use serde_json;

/// Resolve a Value to a serde_json::Value, substituting variables.
pub fn resolve_value(
    value: &Value,
    variables: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    match value {
        Value::String(s) => serde_json::json!(s),
        Value::Int(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::Bool(b) => serde_json::json!(b),
        Value::Null => serde_json::Value::Null,
        Value::Variable(name) => variables
            .get(name)
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        Value::List(items) => {
            let resolved: Vec<_> = items.iter().map(|v| resolve_value(v, variables)).collect();
            serde_json::json!(resolved)
        }
        Value::Object(pairs) => {
            let mut map = serde_json::Map::new();
            for (k, v) in pairs {
                map.insert(k.clone(), resolve_value(v, variables));
            }
            serde_json::Value::Object(map)
        }
    }
}

/// Pick only the requested fields from a JSON object, respecting aliases.
pub fn pick_fields(
    data: &serde_json::Value,
    selections: &[Selection],
    variables: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut result = serde_json::Map::new();

    for sel in selections {
        let response_key = sel.alias.as_deref().unwrap_or(&sel.name);
        let source_key = &sel.name;

        let value = data.get(source_key).cloned().unwrap_or(serde_json::Value::Null);

        let value = if let Some(ref nested_set) = sel.selection_set {
            match &value {
                serde_json::Value::Array(items) => {
                    let mapped: Vec<_> = items
                        .iter()
                        .map(|item| pick_fields(item, &nested_set.selections, variables))
                        .collect();
                    serde_json::json!(mapped)
                }
                serde_json::Value::Object(_) => {
                    pick_fields(&value, &nested_set.selections, variables)
                }
                _ => value,
            }
        } else {
            value
        };

        result.insert(response_key.to_string(), value);
    }

    serde_json::Value::Object(result)
}

/// Build the argument map for a selection, resolving variables.
pub fn build_args(
    selection: &Selection,
    variables: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut args = serde_json::Map::new();
    for arg in &selection.arguments {
        args.insert(arg.name.clone(), resolve_value(&arg.value, variables));
    }
    args
}
```

**Step 4: Add `mod execute;` to lib.rs**

**Step 5: Run tests to verify they pass**

Run: `cargo test -p gql-custom-parser`
Expected: All tests PASS

**Step 6: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: implement execution engine with variable resolution and field picking"
```

---

### Task 7: Implement the HTTP client

**Files:**
- Create: `workers/gql-custom-parser/src/http_client.rs`
- Modify: `workers/gql-custom-parser/src/lib.rs`

Identical in purpose to the async-graphql version — wraps `worker::Fetch`.

**Step 1: Implement http_client.rs**

```rust
use serde::de::DeserializeOwned;
use worker::{Fetch, Url};

pub struct OriginClient {
    base_url: String,
}

impl OriginClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let parsed_url = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

        let mut response = Fetch::Url(parsed_url)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, String> {
        let url = format!("{}{}", self.base_url, path);
        let body_json = serde_json::to_string(body)
            .map_err(|e| format!("Failed to serialize request body: {e}"))?;

        let mut request_init = worker::RequestInit::new();
        request_init.with_method(worker::Method::Post);
        request_init.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body_json)));

        let request = worker::Request::new_with_init(&url, &request_init)
            .map_err(|e| format!("Failed to create request: {e}"))?;

        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {e}"))?;

        if response.status_code() >= 400 {
            return Err(format!("Origin returned status {}", response.status_code()));
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse response JSON: {e}"))
    }
}
```

**Step 2: Add `mod http_client;` to lib.rs**

**Step 3: Verify it compiles**

Run: `cd workers/gql-custom-parser && cargo build --target wasm32-unknown-unknown --release`

**Step 4: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: add OriginClient for upstream HTTP calls"
```

---

### Task 8: Wire everything into the handler

**Files:**
- Modify: `workers/gql-custom-parser/src/handler.rs`

**Step 1: Implement the full handler**

```rust
use worker::*;

use crate::execute::{build_args, pick_fields, resolve_value};
use crate::http_client::OriginClient;
use crate::parser::ast::*;
use crate::parser::parse;
use crate::parser::validate;
use crate::schema::flight_schema;

pub fn health() -> Result<http::Response<String>> {
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(r#"{"status":"ok"}"#.to_string())
        .unwrap())
}

pub async fn graphql(req: HttpRequest, env: Env) -> Result<http::Response<String>> {
    let origin_base_url = match env.var("ORIGIN_BASE_URL") {
        Ok(v) => v.to_string(),
        Err(_) => {
            return Ok(http::Response::builder()
                .status(502)
                .header("content-type", "application/json")
                .body(r#"{"error":"ORIGIN_BASE_URL not configured"}"#.to_string())
                .unwrap());
        }
    };

    let client = OriginClient::new(origin_base_url);

    let body_bytes = req.into_body();
    let body: Vec<u8> = body_bytes.bytes().await.unwrap_or_default();

    let request: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return gql_error_response(&format!("Invalid request body: {e}")),
    };

    let query = match request.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return gql_error_response("Missing 'query' field"),
    };

    let variables: serde_json::Map<String, serde_json::Value> = request
        .get("variables")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    // Parse
    let doc = match parse::parse(query) {
        Ok(d) => d,
        Err(e) => return gql_error_response(&e.to_string()),
    };

    // Validate
    let schema = flight_schema();
    if let Err(e) = validate::validate(&doc, &schema) {
        return gql_error_response(&e.to_string());
    }

    // Execute
    let result = execute_operation(&doc.operation, &variables, &client).await;

    match result {
        Ok(data) => gql_success_response(data),
        Err(e) => gql_error_response(&e),
    }
}

async fn execute_operation(
    op: &Operation,
    variables: &serde_json::Map<String, serde_json::Value>,
    client: &OriginClient,
) -> std::result::Result<serde_json::Value, String> {
    let mut result = serde_json::Map::new();

    for sel in &op.selection_set.selections {
        let response_key = sel.alias.as_deref().unwrap_or(&sel.name);
        let args = build_args(sel, variables);

        let data = match (op.operation_type.clone(), sel.name.as_str()) {
            (OperationType::Query, "flight") => {
                let id = args
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or("flight requires 'id' argument")?;
                let path = format!("/flights/{id}");
                match client.get::<serde_json::Value>(&path).await {
                    Ok(data) => {
                        if let Some(ref ss) = sel.selection_set {
                            pick_fields(&data, &ss.selections, variables)
                        } else {
                            data
                        }
                    }
                    Err(e) if e.contains("404") => serde_json::Value::Null,
                    Err(e) => return Err(e),
                }
            }
            (OperationType::Query, "flights") => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
                let offset = args.get("offset").and_then(|v| v.as_i64()).unwrap_or(0);
                let path = format!("/flights?limit={limit}&offset={offset}");
                let data: Vec<serde_json::Value> = client.get(&path).await?;
                if let Some(ref ss) = sel.selection_set {
                    let mapped: Vec<_> = data
                        .iter()
                        .map(|item| pick_fields(item, &ss.selections, variables))
                        .collect();
                    serde_json::json!(mapped)
                } else {
                    serde_json::json!(data)
                }
            }
            (OperationType::Mutation, "createFlight") => {
                let input = args
                    .get("input")
                    .ok_or("createFlight requires 'input' argument")?;
                let data: serde_json::Value = client.post("/flights", input).await?;
                if let Some(ref ss) = sel.selection_set {
                    pick_fields(&data, &ss.selections, variables)
                } else {
                    data
                }
            }
            (_, name) => return Err(format!("Unknown field: {name}")),
        };

        result.insert(response_key.to_string(), data);
    }

    Ok(serde_json::Value::Object(result))
}

fn gql_success_response(data: serde_json::Value) -> Result<http::Response<String>> {
    let body = serde_json::json!({"data": data});
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .unwrap())
}

fn gql_error_response(message: &str) -> Result<http::Response<String>> {
    let body = serde_json::json!({
        "data": null,
        "errors": [{"message": message}]
    });
    Ok(http::Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .unwrap())
}
```

**Step 2: Verify it compiles**

Run: `cd workers/gql-custom-parser && cargo build --target wasm32-unknown-unknown --release`

**Step 3: Commit**

```bash
git add workers/gql-custom-parser/src/
git commit -m "feat: wire parser, validator, executor, and HTTP client into handler"
```

---

### Task 9: Add GitHub Actions workflow

**Files:**
- Create: `.github/workflows/gql-custom-parser.yml`

**Step 1: Create the workflow file**

```yaml
name: gql-custom-parser

on:
  push:
    branches: [main]
    paths:
      - 'workers/gql-custom-parser/**'
      - 'Cargo.toml'
      - 'rust-toolchain.toml'
  pull_request:
    paths:
      - 'workers/gql-custom-parser/**'
      - 'Cargo.toml'
      - 'rust-toolchain.toml'
  workflow_dispatch:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Cache cargo registry and build
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-custom-parser-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: cargo test -p gql-custom-parser

      - name: Build
        run: cargo build --target wasm32-unknown-unknown --release -p gql-custom-parser

  deploy:
    needs: test
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Deploy to Cloudflare
        working-directory: workers/gql-custom-parser
        run: npx wrangler deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

**Step 2: Commit**

```bash
git add .github/workflows/gql-custom-parser.yml
git commit -m "ci: add GitHub Actions workflow for gql-custom-parser"
```

---

### Task 10: End-to-end smoke test with wrangler dev

**Files:** None — manual verification only.

**Step 1: Start the worker locally**

Run: `cd workers/gql-custom-parser && npx wrangler dev`

**Step 2: Test health endpoint**

```bash
curl http://localhost:8787/health
```
Expected: `{"status":"ok"}`

**Step 3: Test GraphQL query (no origin — expect error)**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights(limit: 5) { id date aircraftTitle } }"}'
```
Expected: `{"data":null,"errors":[{"message":"Fetch failed: ..."}]}`

**Step 4: Test validation error**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights { nonexistent } }"}'
```
Expected: `{"data":null,"errors":[{"message":"Validation error: Field 'nonexistent' not found on type 'Flight'"}]}`

**Step 5: Test parse error**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ flights { "}'
```
Expected: `{"data":null,"errors":[{"message":"Parse error: ..."}]}`

**Step 6: Test alias**

```bash
curl -X POST http://localhost:8787/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ recentFlights: flights(limit: 3) { id date } }"}'
```
Expected: Response uses "recentFlights" as the key (origin error expected, but alias should be in the error path or response key)

**Step 7: Document results and commit any fixes**
