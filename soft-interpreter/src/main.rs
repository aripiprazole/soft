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

use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    panic::Location,
    path::PathBuf,
    rc::Rc,
};

use thiserror::Error;

pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;

pub type Value = Rc<RefCell<Expr>>;

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
}

/// A "stack frame" it stores variables in the stack and it is always a copy of the last one.
#[derive(Debug, Clone)]
pub struct Frame {
    pub name: String,
    pub located_at: Meta,
    pub variables: im_rc::HashMap<String, Value, fxhash::FxBuildHasher>,
}

/// The environment of the interpreter. It contains a stack of frames that are used to store the
/// variables of the program and the functions calls.
#[derive(Debug, Clone)]
pub struct Environment {
    pub frames: im_rc::Vector<Frame>,
}

impl Default for Environment {
    fn default() -> Environment {
        let mut env = Environment {
            frames: im_rc::vector![Frame {
                name: "root".into(),
                located_at: Meta::Unknown,
                variables: Default::default(),
            }],
        };

        env.push_stack("root".into());
        env
    }
}

impl Environment {
    /// Gets the last stack frame.
    pub fn last_stack(&mut self) -> &mut Frame {
        self.frames.back_mut().unwrap()
    }

    /// Extends the current stack frame with a new primitve variable.
    #[track_caller]
    pub fn extend(&mut self, name: &'static str, call: Prim) {
        let location = *Location::caller();
        let current_stack = self.last_stack();

        current_stack.variables.insert(
            name.into(),
            Rc::new(RefCell::new(Expr::Extern(Extern {
                name,
                location,
                call,
            }))),
        );
    }

    /// Adds a new stack frame based on the last one
    pub fn push_stack(&mut self, name: String) -> &mut Frame {
        let frame = Frame {
            name,
            located_at: Meta::Unknown,
            variables: self.last_stack().variables.clone(),
        };
        self.frames.push_back(frame);
        self.frames.back_mut().unwrap()
    }

    pub fn pop_stack(&mut self) {
        self.frames.pop_back();
    }

    pub fn unwind(&mut self) {
        println!("stack backtrace:");
        while let Some(frame) = self.frames.pop_back() {
            match frame.located_at {
                Meta::Location { line, column, file } => {
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

/// Meta information about a value. It can be used to store the location of the value in the source
/// code or the location of the value in the source code.
#[derive(Debug, Clone)]
pub enum Meta {
    Location {
        line: u64,
        column: u64,
        file: Option<PathBuf>,
    },
    Extern(&'static str, Location<'static>),
    Unknown,
}

#[derive(Debug, Clone)]
pub enum Expr {
    // List operators
    Nil,
    Cons(Value, Value),

    // Literal values
    Symbol(String),
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
        Rc::new(RefCell::new(self))
    }

    /// Compare two simple values by value and others by reference.
    pub fn compare(&self, other: &Expr) -> bool {
        match (self, other) {
            (Expr::Nil, Expr::Nil) => true,
            (Expr::Symbol(x), Expr::Symbol(y)) => x == y,
            (Expr::Str(x), Expr::Str(y)) => x == y,
            (Expr::Int(x), Expr::Int(y)) => x == y,
            (Expr::Decimal(x, xs), Expr::Decimal(y, ys)) => x == y && xs == ys,
            (Expr::Meta(_, n), other) | (other, Expr::Meta(_, n)) => n.borrow().compare(other),
            (a, b) => std::ptr::eq(a, b),
        }
    }

    /// Gets the spine of elements of a cons list._
    fn spine(value: Value) -> Vec<Value> {
        let mut spine = Vec::new();
        let mut current = value;

        while let Expr::Cons(tail, head) = &*current.clone().borrow() {
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
            Expr::Meta(_, value) => write!(f, "<{}>", value.borrow()),
            Expr::Symbol(atom) => write!(f, "{}", atom),
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
                    write!(f, "{}", value.borrow())?;
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
                    write!(f, "{}", value.borrow())?;
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
        let value = (self.call)(scope);
        env.pop_stack();
        value
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
            .unwrap_or_else(|| Rc::new(RefCell::new(Expr::Nil)))
    }

    pub fn value(&self, expr: Expr) -> Value {
        Rc::new(RefCell::new(expr))
    }

    pub fn ok(&self, expr: Expr) -> Result<Value> {
        Ok(Rc::new(RefCell::new(expr)))
    }
}

pub trait Eval {
    fn eval(&self, env: &mut Environment) -> Result<Value>;
}

pub struct Application<'a>(Value, &'a [Value]);

impl<'a> Eval for Application<'a> {
    fn eval(&self, env: &mut Environment) -> Result<Value> {
        let head_val = self.0.borrow();

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
        match &*self.borrow() {
            Expr::Cons(_, _) => {
                let spine = Expr::spine(self.clone());
                let head = spine.first().unwrap();
                let tail = &spine[1..];
                Application(head.clone(), tail).eval(env)
            }
            Expr::Symbol(_) => {
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

pub fn parse(code: &str) -> Result<Vec<Value>> {
    let mut peekable = code.chars().peekable();
    let mut stack = vec![];
    let mut indices = vec![];

    while let Some(chr) = peekable.next() {
        match chr {
            ' ' | '\n' | '\t' | '\r' => (),
            '(' => {
                indices.push(stack.len());
            }
            ')' => {
                if let Some(start) = indices.pop() {
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
                    num += peekable.next().unwrap() as u64 - '0' as u64;
                }

                stack.push(Expr::Int(num).to_value());
            }
            chr => {
                let mut symbol = chr.to_string();

                while let Some(chr) = peekable.peek() {
                    if matches!(chr, '(' | ')' | '\n' | '\t' | '\r' | ' ') {
                        break;
                    }
                    symbol.push(peekable.next().unwrap());
                }

                stack.push(Expr::Symbol(symbol).to_value());
            }
        }
    }

    if !indices.is_empty() {
        Err(RuntimeError::UnmatchedParenthesis)
    } else {
        Ok(stack)
    }
}

fn main() {
    panic!("Hello world");
}

#[cfg(test)]
mod tests {
    use std::panic::Location;

    use crate::{parse, Environment, Expr, Meta};

    #[test]
    fn unwind() {
        let mut env = Environment::default();

        env.push_stack("main".to_string()).located_at = Meta::Location {
            column: 10,
            line: 4,
            file: Some("main".into()),
        };

        env.push_stack("fib".to_string()).located_at = Meta::Extern("fib", *Location::caller());
        env.push_stack("error".to_string()).located_at = Meta::Extern("fib", *Location::caller());
        env.push_stack("unknown".to_string()).located_at = Meta::Unknown;
        env.unwind();
    }

    #[test]
    fn environment_abs() {
        let mut env = Environment::default();
        env.extend("nil", |scope| scope.ok(Expr::Nil));
    }

    #[test]
    fn expr_test() {
        let mut env = Environment::default();
        env.extend("nil", |scope| scope.ok(Expr::Nil));

        let result = parse("(1 2 3)");
        println!("{}", result.unwrap().first().unwrap().borrow());
    }
}
