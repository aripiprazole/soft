//! This is the single file interpreter for soft lisp. It contains a Reader, An evaluator, A printer
//! and a simple REPL. The goal is to have a single file interpreter that can be used to bootstrap
//! the language. The interpreter is not optimized for speed, but for simplicity and ease of use.
//! All the values are "garbage collected" using Reference Counting.
//!
//! # Supported Syntax
//! The supported syntax is a subset of the syntax supported by the interpreter. It's not supposed
//! to be a complete soft language tough, It's only supposed to be a subset that can be used to
//! bootstrap the language.
//!
//! ```lisp
//! (set* <name> <value>)
//! (lambda* (<name>*) body)
//! (cons <car> <cdr>)
//! (vec <value>*)
//! (if* <condition> <then> <else>)
//! (quote* <value>)
//! (begin* <expr>*)
//! (call* <function>)
//! (eq* a b)
//! ```

pub mod intrinsics;

use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    iter::Peekable,
    panic::Location,
    path::PathBuf,
    rc::Rc,
    str::Chars,
};

use thiserror::Error;

pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;

/// Runtime errors that can happen during the execution of the program.

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("undefined name {0}")]
    UndefinedName(String),

    #[error("cannot call as function")]
    NotCallable,

    #[error("unmatched parenthesis")]
    UnmatchedParenthesis,

    #[error("unclosed parenthesis")]
    UnclosedParenthesis,

    #[error("unclosed string")]
    UnclosedString,

    #[error("unmatched quote")]
    UnmatchedQuote,

    #[error("wrong arity, expected {0} arguments, got {1}")]
    WrongArity(usize, usize),

    #[error("expected an identifier")]
    ExpectedIdentifier,
}

/// A "stack frame" it stores variables in the stack and it is always a copy of the last one.
#[derive(Debug, Clone)]
pub struct Frame {
    pub name: String,
    pub located_at: Meta,
    pub variables: im_rc::HashMap<String, Value, fxhash::FxBuildHasher>,
    pub catch: bool,
}

impl Frame {
    /// Creates a new root stack frame. It's the first stack frame used to run top level
    /// expressions.
    pub fn root(file: Option<PathBuf>) -> Frame {
        Frame {
            name: "root".into(),
            located_at: Meta::Location(Place {
                line: 0,
                column: 0,
                file,
            }),
            variables: Default::default(),
            catch: true,
        }
    }
}

/// The environment of the interpreter. It contains a stack of frames that are used to store the
/// variables of the program and the functions calls.
#[derive(Debug, Clone)]
pub struct Environment {
    pub frames: im_rc::Vector<Frame>,
}

impl Environment {
    pub fn new(path: Option<PathBuf>) -> Environment {
        Environment {
            frames: im_rc::vector![Frame::root(path)],
        }
    }

    /// Gets the last stack frame.
    pub fn last_stack(&mut self) -> &mut Frame {
        self.frames.back_mut().unwrap()
    }

    /// Extends the current stack frame with a new primitive variable.
    #[track_caller]
    pub fn extend(&mut self, name: &'static str, call: Prim) {
        let location = *Location::caller();
        let current_stack = self.last_stack();

        current_stack.variables.insert(
            name.into(),
            Value(Rc::new(RefCell::new(Expr::Extern(Extern {
                name,
                location,
                call,
            })))),
        );
    }

    pub fn find_first_location(&mut self) -> Meta {
        for frame in self.frames.iter().rev() {
            if let Meta::Location(_) = frame.located_at {
                return frame.located_at.clone();
            }
        }
        self.last_stack().located_at.clone()
    }

    /// Adds a new stack frame based on the last one
    pub fn push_stack(&mut self, name: String) -> &mut Frame {
        let frame = Frame {
            name,
            located_at: Meta::Unknown,
            variables: self.last_stack().variables.clone(),
            catch: false,
        };
        self.frames.push_back(frame);
        self.frames.back_mut().unwrap()
    }

    pub fn pop_stack(&mut self) {
        self.frames.pop_back();
    }

    pub fn unwind(&mut self) {
        println!("stack backtrace:");
        while self.frames.last().map(|x| !x.catch).unwrap_or(false) {
            let frame = self.frames.pop_back().unwrap();
            match frame.located_at {
                Meta::Location(Place { line, column, file }) => {
                    let file = match file {
                        Some(file) => file.display().to_string(),
                        None => "REPL".into(),
                    };

                    println!("  at {}", frame.name);
                    println!("    {}:{}:{}", file, line, column)
                }
                Meta::Extern(name, location) => {
                    let file = location.file();
                    let line = location.line();
                    let column = location.column();

                    println!("  at external/{name}",);
                    println!("    {}:{}:{}", file, line, column)
                }
                Meta::Unknown => {
                    println!("  at <unknown>")
                }
            }
        }
    }

    /// Gets a variable from the last stack frame
    pub fn get(&mut self, name: &str) -> Option<Value> {
        self.last_stack().variables.get(name).cloned()
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.last_stack().variables.insert(name, value);
    }
}

#[derive(Debug, Clone)]
pub struct Value(Rc<RefCell<Expr>>);

impl Value {
    pub fn located(self, meta: Meta) -> Value {
        Expr::Meta(meta, self).to_value()
    }
    /// Compare two simple values by value and others by reference.
    pub fn compare(&self, other: &Value) -> bool {
        match (&*self.0.borrow(), &*other.0.borrow()) {
            (Expr::Nil, Expr::Nil) => true,
            (Expr::Identifier(x), Expr::Identifier(y)) => x == y,
            (Expr::Str(x), Expr::Str(y)) => x == y,
            (Expr::Int(x), Expr::Int(y)) => x == y,
            (Expr::Decimal(x, xs), Expr::Decimal(y, ys)) => x == y && xs == ys,
            (Expr::Meta(_, n), _) => n.compare(self),
            (_, Expr::Meta(_, n)) => self.compare(n),
            (a, b) => std::ptr::eq(a, b),
        }
    }

    pub fn assert_size(&self, size: usize) -> Result<Vec<Value>> {
        match &*self.0.borrow() {
            Expr::Cons(..) => {
                let spine = Expr::spine(self.clone());
                if spine.len() == size {
                    Ok(spine)
                } else {
                    Err(RuntimeError::WrongArity(spine.len(), size))
                }
            }
            _ => Err(RuntimeError::WrongArity(size, 1)),
        }
    }

    pub fn at_least(&self, size: usize) -> Result<Vec<Value>> {
        match &*self.0.borrow() {
            Expr::Cons(..) => {
                let spine = Expr::spine(self.clone());
                if spine.len() >= size {
                    Ok(spine)
                } else {
                    Err(RuntimeError::WrongArity(spine.len(), size))
                }
            }
            _ => Err(RuntimeError::WrongArity(size, 1)),
        }
    }

    pub fn assert_identifier(&self) -> Result<String> {
        match &*self.0.borrow() {
            Expr::Identifier(name) => Ok(name.clone()),
            _ => Err(RuntimeError::ExpectedIdentifier),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

#[derive(Debug, Clone)]
pub struct Place {
    line: u64,
    column: u64,
    file: Option<PathBuf>,
}

/// Meta information about a value. It can be used to store the location of the value in the source
/// code or the location of the value in the source code.
#[derive(Debug, Clone)]
pub enum Meta {
    Location(Place),
    Extern(&'static str, Location<'static>),
    Unknown,
}

impl Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Meta::Location(Place {
                line,
                column,
                file: Some(file),
            }) => write!(f, "{}:{}:{}", file.display(), line, column),
            Meta::Location(Place { line, column, .. }) => write!(f, "REPL:{}:{}", line, column),
            Meta::Extern(name, _) => write!(f, "external/{}", name),
            Meta::Unknown => write!(f, "<unknown>"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    // List operators
    Nil,
    Cons(Value, Value),

    // Literal values
    Identifier(String),
    Atom(String),
    Str(String),
    Int(u64),
    Vector(Vec<Value>),
    Decimal(u64, u64),

    // Function things
    Extern(Extern),
    Closure(Closure),

    Meta(Meta, Value),
}

impl Expr {
    pub fn to_value(self) -> Value {
        Value(Rc::new(RefCell::new(self)))
    }

    /// Gets the spine of elements of a cons list._
    fn spine(value: Value) -> Vec<Value> {
        let mut spine = Vec::new();
        let mut current = value;

        while let Expr::Cons(tail, head) = &*current.clone().0.borrow() {
            spine.push(head.clone());
            current = tail.clone();
        }

        spine.push(current);
        spine.reverse();
        spine
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Nil => write!(f, "<nil>"),
            Expr::Meta(_, value) => write!(f, "<{}>", value.0.borrow()),
            Expr::Identifier(atom) => write!(f, "{}", atom),
            Expr::Atom(atom) => write!(f, "'{}", atom),
            Expr::Str(string) => write!(f, "\"{}\"", string),
            Expr::Int(int) => write!(f, "{}", int),
            Expr::Decimal(int, dec) => write!(f, "{}.{}", int, dec),
            Expr::Cons(..) => {
                let spine = Expr::spine(self.clone().to_value());
                write!(f, "(")?;
                for (i, value) in spine.iter().enumerate() {
                    if i != 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", value.0.borrow())?;
                }
                write!(f, ")")
            }
            Expr::Extern(value) => write!(f, "<extern {}>", value.name),
            Expr::Closure(_) => write!(f, "<closure>"),
            Expr::Vector(vec) => {
                write!(f, "[")?;
                for (i, value) in vec.iter().enumerate() {
                    if i != 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", value.0.borrow())?;
                }
                write!(f, "]")
            }
        }
    }
}

pub trait Function {
    fn call(&self, env: &mut Environment, args: Vec<Value>) -> Result<Value>;
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub env: Environment,
    pub meta: Meta,
    pub name: Option<String>,
    pub params: Vec<String>,
    pub value: Value,
}

impl Function for Closure {
    fn call(&self, old_env: &mut Environment, args: Vec<Value>) -> Result<Value> {
        let name = self.name.clone().unwrap_or_default();
        let env = &mut self.env.clone();

        let frame = env.push_stack(format!("<closure:{}>", name));
        frame.located_at = self.meta.clone();

        let params = self.params.iter().cloned();
        let args = args.into_iter();

        for (param, arg) in params.zip(args) {
            env.set(param, arg.eval(old_env)?);
        }

        let value = self.value.eval(env)?;

        env.pop_stack();
        Ok(value)
    }
}

pub type Prim = fn(CallScope<'_>) -> Result<Value>;

#[derive(Clone)]
pub struct Extern {
    name: &'static str,
    location: Location<'static>,
    call: Prim,
}

impl Debug for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extern").finish()
    }
}

impl Function for Extern {
    fn call(&self, env: &mut Environment, args: Vec<Value>) -> Result<Value> {
        let frame = env.push_stack(self.name.to_string());
        frame.located_at = Meta::Extern(self.name, self.location);
        let scope = CallScope { args, env };
        let value = (self.call)(scope)?;
        env.pop_stack();
        Ok(value)
    }
}

pub struct CallScope<'a> {
    pub args: Vec<Value>,
    pub env: &'a mut Environment,
}

impl CallScope<'_> {
    pub fn at(&self, nth: usize) -> Value {
        self.args
            .get(nth)
            .cloned()
            .unwrap_or_else(|| Expr::Nil.to_value())
    }

    pub fn value(&self, expr: Expr) -> Value {
        expr.to_value()
    }

    pub fn assert_size(&self, size: usize) -> Result<()> {
        if self.args.len() != size {
            Err(RuntimeError::WrongArity(size, self.args.len()))
        } else {
            Ok(())
        }
    }

    pub fn assert_at_least(&self, size: usize) -> Result<()> {
        if self.args.len() < size {
            Err(RuntimeError::WrongArity(size, self.args.len()))
        } else {
            Ok(())
        }
    }

    pub fn ok(&self, expr: Expr) -> Result<Value> {
        Ok(Value(Rc::new(RefCell::new(expr))))
    }
}

pub trait Eval {
    fn eval(&self, env: &mut Environment) -> Result<Value>;
}

pub struct Application<'a>(Value, &'a [Value]);

impl<'a> Eval for Application<'a> {
    fn eval(&self, env: &mut Environment) -> Result<Value> {
        let head_val = self.0.eval(env)?;
        let head_val = head_val.0.borrow();

        let func: &dyn Function = match &*head_val {
            Expr::Extern(ext) => ext,
            Expr::Closure(val) => val,
            _ => return Err(RuntimeError::NotCallable),
        };

        func.call(env, self.1.to_vec())
    }
}

impl Eval for Value {
    fn eval(&self, env: &mut Environment) -> Result<Value> {
        match &*self.0.borrow() {
            Expr::Cons(_, _) => {
                let spine = Expr::spine(self.clone());
                let head = spine.first().unwrap();
                let tail = &spine[1..];
                Application(head.clone(), tail).eval(env)
            }
            Expr::Identifier(_) => {
                let call = env.get("call").unwrap();
                Application(call, &[self.clone()]).eval(env)
            }
            Expr::Meta(loc, expr) => {
                env.last_stack().located_at = loc.clone();
                expr.eval(env)
            }
            _ => Ok(self.clone()),
        }
    }
}

/// Parses a source code at vector of values.
pub fn parse(code: &str, file: Option<PathBuf>) -> Result<Vec<Value>> {
    let mut peekable = code.chars().peekable();
    let mut stack = vec![];
    let mut indices = vec![];
    let mut prefix = vec![];

    let mut meta = Place {
        line: 1,
        column: 0,
        file,
    };

    fn next(peekable: &mut Peekable<Chars<'_>>, meta: &mut Place) -> Option<char> {
        let chr = peekable.next()?;
        match chr {
            '\n' => {
                meta.line += 1;
                meta.column = 0;
            }
            _ => {
                meta.column += 1;
            }
        }
        Some(chr)
    }

    while let Some(chr) = next(&mut peekable, &mut meta) {
        let start_meta = meta.clone();
        match chr {
            ' ' | '\n' | '\t' | '\r' => continue,
            '(' => {
                indices.push((stack.len(), meta.clone()));
                continue;
            }
            ')' => {
                if let Some((start, start_meta)) = indices.pop() {
                    let args = stack.split_off(start);
                    let head = args.first().cloned().unwrap();

                    stack.push(
                        args.into_iter()
                            .skip(1)
                            .fold(head, |x, y| Expr::Cons(x, y).to_value()),
                    );
                } else {
                    return Err(RuntimeError::UnmatchedParenthesis);
                }
            }
            '0'..='9' => {
                let mut num = chr as u64 - '0' as u64;

                while let Some('0'..='9') = peekable.peek() {
                    num *= 10;
                    num += next(&mut peekable, &mut meta).unwrap() as u64 - '0' as u64;
                }

                stack.push(Expr::Int(num).to_value());
            }
            '\'' => {
                prefix.push(stack.len());
                continue;
            }
            chr => {
                let mut symbol = chr.to_string();

                while let Some(chr) = peekable.peek() {
                    if matches!(chr, '(' | ')' | '\n' | '\t' | '\r' | ' ') {
                        break;
                    }
                    symbol.push(peekable.next().unwrap());
                }

                stack.push(Expr::Identifier(symbol).to_value());
            }
        }

        if let Some(start) = prefix.last() {
            if start + 1 == stack.len() {
                prefix.pop();
                let last = stack.pop().unwrap();
                stack.push(
                    Expr::Cons(Expr::Identifier("quote".to_string()).to_value(), last).to_value(),
                );
            }
        }
    }

    if !prefix.is_empty() {
        Err(RuntimeError::UnmatchedQuote)
    } else if !indices.is_empty() {
        Err(RuntimeError::UnmatchedParenthesis)
    } else {
        Ok(stack)
    }
}

#[cfg(test)]
mod tests {
    use std::panic::Location;

    use crate::{parse, Environment, Expr, Meta, Place};

    #[test]
    fn unwind() {
        let mut env = Environment::new(None);

        env.push_stack("main".to_string()).located_at = Meta::Location(Place {
            column: 10,
            line: 4,
            file: Some("main".into()),
        });

        env.push_stack("fib".to_string()).located_at = Meta::Extern("fib", *Location::caller());
        env.push_stack("error".to_string()).located_at = Meta::Extern("fib", *Location::caller());
        env.push_stack("unknown".to_string()).located_at = Meta::Unknown;
        env.unwind();
    }

    #[test]
    fn environment_abs() {
        let mut env = Environment::new(None);
        env.extend("nil", |scope| scope.ok(Expr::Nil));
    }

    #[test]
    fn expr_test() {
        let mut env = Environment::new(None);
        env.extend("nil", |scope| scope.ok(Expr::Nil));

        let result = parse("(1 '2)", None);
        println!("{}", result.unwrap().first().unwrap().0.borrow());
    }
}
