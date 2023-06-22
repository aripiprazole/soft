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
    cell::{Ref, RefCell},
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
    #[error("undefined name '{0}'")]
    UndefinedName(String),

    #[error("cannot call as function '{0}'")]
    NotCallable(Value),

    #[error("unmatched parenthesis")]
    UnmatchedParenthesis(Place),

    #[error("unclosed parenthesis")]
    UnclosedParenthesis(Place),

    #[error("unclosed string")]
    UnclosedString(Place),

    #[error("unmatched quote")]
    UnmatchedQuote(Place),

    #[error("wrong arity, expected {0} arguments, got {1}")]
    WrongArity(usize, usize),

    #[error("expected an identifier but got '{0}'")]
    ExpectedIdentifier(String),

    #[error("expected a list but got '{0}'")]
    ExpectedList(String),

    #[error("expected a number but got '{0}'")]
    ExpectedNumber(String),

    #[error("invalid escape")]
    InvalidEscape,

    #[error("unterminated string")]
    UnterminatedString,
}

impl RuntimeError {
    pub fn get_location(self) -> Option<Meta> {
        let place = match self {
            RuntimeError::UnmatchedParenthesis(place)
            | RuntimeError::UnclosedParenthesis(place)
            | RuntimeError::UnclosedString(place)
            | RuntimeError::UnmatchedQuote(place) => Some(place),
            _ => None,
        };
        place.map(Meta::Location)
    }
}

/// A "stack frame" it stores variables in the stack and it is always a copy of the last one.
#[derive(Debug, Clone)]
pub struct Frame {
    pub name: String,
    pub located_at: Meta,
    pub variables: im_rc::HashMap<String, Value, fxhash::FxBuildHasher>,
    pub is_macro: im_rc::HashSet<String, fxhash::FxBuildHasher>,
    pub catch: bool,
    pub local: bool,
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
            is_macro: Default::default(),
            catch: true,
            local: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Mode {
    Eval,
    Macro,
}

/// The environment of the interpreter. It contains a stack of frames that are used to store the
/// variables of the program and the functions calls.
#[derive(Debug, Clone)]
pub struct Environment {
    pub frames: im_rc::Vector<Frame>,
    pub global: Rc<RefCell<Frame>>,
    pub expanded: bool,
    pub old_env: Option<Box<Environment>>,
    pub mode: Mode,
}

impl Environment {
    pub fn walk_env(&self) -> Environment {
        let mut env = self;
        while let Some(old_env) = &env.old_env {
            env = old_env;
        }
        env.clone()
    }

    pub fn new(path: Option<PathBuf>) -> Environment {
        Environment {
            frames: im_rc::vector![Frame::root(path.clone())],
            global: Rc::new(RefCell::new(Frame::root(path))),
            expanded: false,
            mode: Mode::Macro,
            old_env: None,
        }
    }

    pub fn to_eval(&mut self) {
        self.mode = Mode::Eval;
    }

    pub fn register_intrinsics(&mut self) {
        self.intrinsic("is-cons", crate::intrinsics::is_cons);
        self.intrinsic("call", crate::intrinsics::call);
        self.intrinsic("set*", crate::intrinsics::set);
        self.intrinsic("setm*", crate::intrinsics::setm);
        self.intrinsic("list", crate::intrinsics::list);
        self.intrinsic("lambda*", crate::intrinsics::lambda);
        self.intrinsic("let*", crate::intrinsics::let_);
        self.intrinsic("+", crate::intrinsics::add);
        self.intrinsic("-", crate::intrinsics::sub);
        self.intrinsic("*", crate::intrinsics::mul);
        self.intrinsic("len", crate::intrinsics::len);
        self.intrinsic("quote", crate::intrinsics::quote);
        self.intrinsic("print", crate::intrinsics::print);
        self.intrinsic("cons", crate::intrinsics::cons);
        self.intrinsic("nil", crate::intrinsics::nil);
        self.intrinsic("<", crate::intrinsics::less);
        self.intrinsic("if", crate::intrinsics::if_);
        self.intrinsic("block", crate::intrinsics::block);
        self.intrinsic("head", crate::intrinsics::head);
        self.intrinsic("tail", crate::intrinsics::tail);
        self.intrinsic("eq", crate::intrinsics::eq);
        //self.intrinsic("defn", crate::intrinsics::defn);
    }

    /// Gets the last stack frame.
    pub fn last_stack(&mut self) -> &mut Frame {
        self.frames.back_mut().unwrap()
    }

    /// Add an intrinsic function
    pub fn intrinsic(&mut self, name: &'static str, call: Prim) {
        let mut frame = self.global.borrow_mut();
        frame
            .variables
            .insert(name.into(), Expr::Extern(Extern { name, call }).to_value());
    }

    /// Extends the current stack frame with a new primitive variable.
    #[track_caller]
    pub fn extend(&mut self, name: &'static str, call: Prim) {
        let mut current_stack = self.global.borrow_mut();

        current_stack
            .variables
            .insert(name.into(), Expr::Extern(Extern { name, call }).to_value());
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
            located_at: Meta::Intrinsic,
            variables: self.last_stack().variables.clone(),
            is_macro: self.last_stack().is_macro.clone(),
            catch: false,
            local: false,
        };
        self.frames.push_back(frame);
        self.frames.back_mut().unwrap()
    }

    pub fn add_local_stack(&mut self) {
        let current_stack = self.last_stack().clone();
        self.last_stack().local = true;
        self.frames.push_back(current_stack);
    }

    pub fn pop_stack(&mut self) {
        self.frames.pop_back();
    }

    pub fn print_stack_trace(&mut self, unwinded: Vec<Frame>) {
        println!("\n stack trace:");
        for frame in unwinded {
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
                Meta::Intrinsic => {
                    // Do not print to avoid intrinsic
                    // println!("  at <unknown>")
                }
            }
        }
    }

    pub fn unwind(&mut self) -> Vec<Frame> {
        let mut unwinded = vec![];

        let mut env = self.walk_env();

        while env.frames.last().map(|x| !x.catch).unwrap_or(false) {
            let frame = env.frames.pop_back().unwrap();

            if frame.local {
                continue;
            }

            match frame.located_at {
                Meta::Location(..) | Meta::Extern(..) => {
                    unwinded.push(frame.clone());
                }
                _ => (),
            }
        }
        unwinded
    }

    /// Gets a variable from the last stack frame
    pub fn get(&mut self, name: &str) -> Option<Value> {
        self.last_stack()
            .variables
            .get(name)
            .cloned()
            .or_else(|| self.global.borrow().variables.get(name).cloned())
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.last_stack().variables.insert(name, value);
    }
}

#[derive(Clone)]
pub struct Value(Rc<RefCell<Expr>>, Meta);

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({:#?})", self.0.borrow())
    }
}

impl Value {
    pub fn from_list(list: Vec<Value>) -> Value {
        list.into_iter().rfold(Expr::Nil.to_value(), |next, acc| {
            Expr::Cons(acc, next).to_value()
        })
    }

    pub fn location(&self) -> Option<Meta> {
        match self.1.clone() {
            value @ Meta::Extern(..) | value @ Meta::Location(..) => Some(value),
            _ => None,
        }
    }

    pub fn expr(&self) -> Ref<'_, Expr> {
        self.0.borrow()
    }

    /// Compare two simple values by value and others by reference.
    pub fn compare(&self, other: &Value) -> bool {
        match (&*self.0.borrow(), &*other.0.borrow()) {
            (Expr::Nil, Expr::Nil) => true,
            (Expr::Identifier(x), Expr::Identifier(y)) => x == y,
            (Expr::Str(x), Expr::Str(y)) => x == y,
            (Expr::Int(x), Expr::Int(y)) => x == y,
            (a, b) => std::ptr::eq(a, b),
        }
    }

    pub fn is_true(&self) -> bool {
        match &*self.0.borrow() {
            Expr::Nil => false,
            Expr::Str(x) => !x.is_empty(),
            Expr::Int(n) => *n != 0,
            Expr::Vector(v) => !v.is_empty(),
            Expr::Atom(s) => s != "false",
            Expr::Identifier(s) => s != "false",
            _ => false,
        }
    }

    pub fn assert_size(&self, size: usize) -> Result<Vec<Value>> {
        if let Expr::Cons(..) = &*self.0.borrow() {
            let spine = Expr::spine(self.clone());
            if let Some(spine) = spine {
                if spine.len() == size {
                    return Ok(spine);
                }
                return Err(RuntimeError::WrongArity(spine.len(), size));
            }
        }
        Err(RuntimeError::WrongArity(size, 1))
    }

    pub fn at_least(&self, size: usize) -> Result<Vec<Value>> {
        if let Expr::Cons(..) = &*self.0.borrow() {
            if let Some(spine) = Expr::spine(self.clone()) {
                if spine.len() >= size {
                    return Ok(spine);
                }
                return Err(RuntimeError::WrongArity(spine.len(), size));
            }
        }
        Err(RuntimeError::WrongArity(size, 1))
    }

    pub fn assert_identifier(&self) -> Result<String> {
        match &*self.0.borrow() {
            Expr::Identifier(name) => Ok(name.clone()),
            _ => Err(RuntimeError::ExpectedIdentifier(self.to_string())),
        }
    }

    pub fn assert_number(&self) -> Result<u64> {
        match &*self.0.borrow() {
            Expr::Int(value) => Ok(*value),
            _ => Err(RuntimeError::ExpectedNumber(self.to_string())),
        }
    }

    pub fn assert_list(&self) -> Result<Vec<Value>> {
        match &*self.0.borrow() {
            Expr::Cons(..) => {
                if let Some(spine) = Expr::spine(self.clone()) {
                    Ok(spine)
                } else {
                    Err(RuntimeError::ExpectedList(self.to_string()))
                }
            }
            _ => Err(RuntimeError::ExpectedList(self.to_string())),
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
    Intrinsic,
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
            Meta::Intrinsic => write!(f, "<unknown>"),
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

    // Function things
    Extern(Extern),
    Closure(Closure),
}

impl Expr {
    pub fn to_value(self) -> Value {
        Value(Rc::new(RefCell::new(self)), Meta::Intrinsic)
    }

    pub fn to_meta_value(self, meta: Meta) -> Value {
        Value(Rc::new(RefCell::new(self)), meta)
    }

    /// Gets the spine of elements of a cons list._
    fn spine(value: Value) -> Option<Vec<Value>> {
        let mut spine = Vec::new();
        let mut current = value;

        while let Expr::Cons(head, tail) = &*current.clone().0.borrow() {
            spine.push(head.clone());
            current = tail.clone();
        }

        let borrow = current.0.borrow();
        match &*borrow {
            Expr::Nil => Some(spine),
            _ => None,
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Nil => write!(f, "<nil>"),
            Expr::Identifier(atom) => write!(f, "{}", atom),
            Expr::Atom(atom) => write!(f, "'{}", atom),
            Expr::Str(string) => write!(f, "\"{}\"", string),
            Expr::Int(int) => write!(f, "{}", int),
            Expr::Cons(head, tail) => {
                let spine = Expr::spine(self.clone().to_value());
                if let Some(spine) = spine {
                    write!(f, "(")?;
                    for (i, value) in spine.iter().enumerate() {
                        if i != 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", value.0.borrow())?;
                    }
                    write!(f, ")")
                } else {
                    write!(f, "({head} . {tail})")
                }
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

impl From<Closure> for Expr {
    fn from(value: Closure) -> Self {
        Expr::Closure(value)
    }
}

impl Function for Closure {
    fn call(&self, old_env: &mut Environment, args: Vec<Value>) -> Result<Value> {
        let name = self.name.clone().unwrap_or_default();
        let env = &mut self.env.clone();

        let params = self.params.iter().cloned();
        let args = args.into_iter();

        let evaluated = args.map(|x| x.eval(old_env)).collect::<Result<Vec<_>>>()?;

        for (param, arg) in params.zip(evaluated) {
            env.set(param, arg);
        }

        let frame = env.push_stack(format!("<closure:{}>", name));
        frame.located_at = self.meta.clone();

        let value = match self.value.eval(env) {
            Ok(value) => value,
            Err(err) => {
                old_env.old_env = Some(Box::new(env.clone()));
                return Err(err);
            }
        };

        env.pop_stack();
        Ok(value)
    }
}

pub type Prim = fn(CallScope<'_>) -> Result<Value>;

#[derive(Clone)]
pub struct Extern {
    name: &'static str,
    call: Prim,
}

impl From<Extern> for Expr {
    fn from(value: Extern) -> Self {
        Expr::Extern(value)
    }
}

impl Debug for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Extern").finish()
    }
}

impl Function for Extern {
    fn call(&self, env: &mut Environment, args: Vec<Value>) -> Result<Value> {
        let scope = CallScope { args, env };
        let value = (self.call)(scope)?;
        Ok(value)
    }
}

impl<'a> From<&'a [Value]> for Expr {
    fn from(value: &'a [Value]) -> Self {
        Expr::Vector(value.into())
    }
}

impl From<Vec<Value>> for Expr {
    fn from(value: Vec<Value>) -> Self {
        Expr::Vector(value)
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

    pub fn assert_arity(&self, size: usize) -> Result<()> {
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

    pub fn ok<T: Into<Expr>>(&self, expr: T) -> Result<Value> {
        Ok(Value(Rc::new(RefCell::new(expr.into())), Meta::Intrinsic))
    }
}

pub trait Eval {
    fn eval(&self, env: &mut Environment) -> Result<Value>;
}

pub struct Application<'a>(Value, Value, &'a [Value]);

impl<'a> Eval for Application<'a> {
    fn eval(&self, env: &mut Environment) -> Result<Value> {
        let value = self.1.eval(env)?;
        let head = value.0.borrow();

        let func: &dyn Function = match &*head {
            Expr::Extern(ext) => ext,
            Expr::Closure(val) => val,
            _ if env.mode == Mode::Macro => {
                let mut args = self
                    .2
                    .iter()
                    .map(|x| x.eval(env))
                    .collect::<Result<Vec<_>>>()?;
                args.insert(0, value.clone());

                let value = args
                    .into_iter()
                    .try_rfold(Expr::Nil.to_value(), |next, acc| {
                        Ok(Expr::Cons(acc, next).to_value())
                    })?;

                return Ok(value);
            }
            _ => return Err(RuntimeError::NotCallable(value.clone())),
        };

        func.call(env, self.2.to_vec())
    }
}

impl Eval for Value {
    fn eval(&self, env: &mut Environment) -> Result<Value> {
        if let Some(location) = self.location() {
            env.last_stack().located_at = location;
        }

        match &*self.0.borrow() {
            Expr::Cons(..) => {
                let spine = Expr::spine(self.clone());
                if let Some(spine) = spine {
                    let head = spine.first().unwrap();
                    let tail = &spine[1..];
                    Application(self.clone(), head.clone(), tail).eval(env)
                } else {
                    Ok(self.clone())
                }
            }
            Expr::Identifier(..) => {
                let call = env.get("call").unwrap();
                Application(self.clone(), call, &[self.clone()]).eval(env)
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
        let place = meta.clone();
        match chr {
            ';' => {
                while let Some(chr) = next(&mut peekable, &mut meta) {
                    if chr == '\n' {
                        break;
                    }
                }
            }
            ' ' | '\n' | '\t' | '\r' => continue,
            '(' => {
                indices.push((stack.len(), meta.clone()));
                continue;
            }
            ')' => {
                if let Some((start, meta)) = indices.pop() {
                    let args = stack.split_off(start);

                    stack.push(args.into_iter().rfold(Expr::Nil.to_value(), |y, x| {
                        Expr::Cons(x, y).to_meta_value(Meta::Location(meta.clone()))
                    }));
                } else {
                    return Err(RuntimeError::UnmatchedParenthesis(place));
                }
            }
            '"' => {
                let mut string = String::new();

                while let Some(chr) = next(&mut peekable, &mut meta) {
                    match chr {
                        '"' => break,
                        '\\' => {
                            let chr = next(&mut peekable, &mut meta).unwrap();
                            match chr {
                                'n' => string.push('\n'),
                                't' => string.push('\t'),
                                'r' => string.push('\r'),
                                '\\' => string.push('\\'),
                                '"' => string.push('"'),
                                _ => return Err(RuntimeError::InvalidEscape),
                            }
                        }
                        chr => string.push(chr),
                    }
                }

                if peekable.peek().is_none() {
                    return Err(RuntimeError::UnterminatedString);
                }

                stack.push(Expr::Str(string).to_meta_value(Meta::Location(place)));
            }
            '0'..='9' => {
                let mut num = chr as u64 - '0' as u64;

                while let Some('0'..='9') = peekable.peek() {
                    num *= 10;
                    num += next(&mut peekable, &mut meta).unwrap() as u64 - '0' as u64;
                }

                stack.push(Expr::Int(num).to_meta_value(Meta::Location(place)));
            }
            '\'' => {
                prefix.push(("quote", indices.len()));
                continue;
            }

            ',' => {
                prefix.push(("unquote", indices.len()));
                continue;
            }
            chr => {
                let mut symbol = chr.to_string();

                while let Some(chr) = peekable.peek() {
                    if matches!(chr, '(' | ')' | '\n' | '\t' | '\r' | ' ' | '"') {
                        break;
                    }
                    symbol.push(next(&mut peekable, &mut meta).unwrap());
                }

                stack.push(Expr::Identifier(symbol).to_meta_value(Meta::Location(place)));
            }
        }

        if let Some((name, start)) = prefix.last() {
            if *start == indices.len() {
                let name = name.to_string();
                prefix.pop();
                let last = stack.pop().unwrap();
                stack.push(
                    Expr::Cons(
                        Expr::Identifier(name).to_meta_value(last.1.clone()),
                        Expr::Cons(last, Expr::Nil.to_value()).to_value(),
                    )
                    .to_value(),
                );
            }
        }
    }

    if !prefix.is_empty() {
        Err(RuntimeError::UnmatchedQuote(meta))
    } else if !indices.is_empty() {
        Err(RuntimeError::UnmatchedParenthesis(meta))
    } else {
        Ok(stack)
    }
}

pub fn expand(env: &mut Environment, mut expr: Value) -> Result<Value, RuntimeError> {
    env.mode = Mode::Macro;
    env.expanded = true;

    while env.expanded {
        env.expanded = false;
        expr = expr.eval(env)?;
    }

    Ok(expr)
}

pub fn run(env: &mut Environment, expr: Value) -> Result<Value, RuntimeError> {
    let expr = expand(env, expr)?;
    env.mode = Mode::Eval;
    expr.eval(env)
}

#[cfg(test)]
mod tests {
    use std::panic::Location;

    use crate::{parse, Environment, Eval, Expr, Meta, Place};

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
        env.push_stack("unknown".to_string()).located_at = Meta::Intrinsic;
        let unwinded = env.unwind();
        env.print_stack_trace(unwinded);
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

    #[test]
    fn repl_test() {
        let mut env = Environment::new(None);
        env.register_intrinsics();

        for expr in parse("((lambda* (x y) (+ x x y y)) 1 2)", None).unwrap() {
            match expr.eval(&mut env) {
                Ok(value) => println!("=> {}", value),
                Err(err) => {
                    eprintln!("{}", expr);
                    eprintln!("error: {err}");
                    eprintln!("  at {}", env.walk_env().last_stack().located_at);
                    let unwinded = env.unwind();
                    env.print_stack_trace(unwinded);
                }
            }
        }
    }
}
