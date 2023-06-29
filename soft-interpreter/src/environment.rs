//! An environment is the context in which an expression is evaluated. It contains a stack of call
//! frames that are used for each function and each [Frame] contains a stack of scopes that are used
//! for each block.

use std::path::PathBuf;

use crate::{error::RuntimeError, value::Location};

use crate::{
    intrinsics,
    value::{Expr, Function::Extern, Prim, Value},
};

/// A frame is an abstraction for a function call space that contains a lot of information about
/// the lexical scope of the function.
#[derive(Debug, Clone)]
pub struct Frame {
    pub name: Option<String>,
    pub location: Location,
    pub catch: bool,
    pub stack: im_rc::Vector<im_rc::HashMap<String, Value>>,
}

impl Frame {
    pub fn new(name: Option<String>, catch: bool, location: Location) -> Self {
        Self {
            name,
            catch,
            stack: im_rc::vector![im_rc::HashMap::new()],
            location,
        }
    }

    /// Tries to find a variable in the current frame or in the global scope.
    pub fn find(&self, id: &str) -> Option<Value> {
        self.stack
            .iter()
            .rev()
            .find_map(|frame| frame.get(id))
            .cloned()
    }

    /// Inserts a variable in the current frame.
    pub fn insert(&mut self, id: String, value: Value) {
        self.stack.back_mut().unwrap().insert(id, value);
    }

    /// Pushes a new scope in the current frame.
    pub fn push(&mut self) {
        self.stack.push_back(im_rc::HashMap::new());
    }

    /// Pops the current scope from the current frame.
    pub fn pop(&mut self) {
        self.stack.pop_back();
    }
}

#[derive(Clone)]
pub struct Def {
    pub name: String,
    pub value: Value,
    pub is_macro: bool,
}

impl From<Def> for Value {
    fn from(def: Def) -> Self {
        let tag = if def.is_macro { "macro" } else { "def" };
        Value::from(Expr::Vector(vec![
            Expr::Atom(def.name.to_string()).into(),
            Expr::Atom(tag.to_string()).into(),
            def.value,
        ]))
    }
}

/// An environment is the context in which an expression is evaluated. It contains a stack of calls
/// and the global environment.
pub struct Environment {
    frames: Vec<Frame>,
    pub global: Value,
    pub expanded: bool,
    pub location: Location,
    pub imported_files: im_rc::HashSet<PathBuf>,
}

impl Environment {
    pub fn new(file: Option<String>) -> Self {
        let start = Location {
            line: 1,
            column: 0,
            file,
        };

        Self {
            frames: vec![Frame::new(None, false, start.clone())],
            expanded: false,
            global: Expr::HashMap(Default::default()).into(),
            location: start,
            imported_files: Default::default(),
        }
    }

    pub fn set_location(&mut self, location: Option<Location>) {
        if let Some(location) = location {
            self.location = location;
        }
    }

    pub fn last_frame(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    pub fn register_intrinsics(&mut self) {
        self.register_external("head", intrinsics::head);
        self.register_external("tail", intrinsics::tail);
        self.register_external("cons", intrinsics::cons);
        self.register_external("list", intrinsics::list);

        self.register_external("read", intrinsics::read);
        self.register_external("flush", intrinsics::flush);
        self.register_external("print", intrinsics::print);
        self.register_external("import", intrinsics::import);

        self.register_external("and", intrinsics::and);
        self.register_external("or", intrinsics::or);

        self.register_external("fn*", intrinsics::fn_);
        self.register_external("if", intrinsics::if_);
        self.register_external("let", intrinsics::letm);
        self.register_external("letrec", intrinsics::letrec);
        self.register_external("set*", intrinsics::set);
        self.register_external("setm*", intrinsics::setm);
        self.register_external("quote", intrinsics::quote);
        self.register_external("expand", intrinsics::expand);
        self.register_external("block", intrinsics::block);
        self.register_external("apply", intrinsics::apply);
        self.register_external("eval", intrinsics::eval);
        self.register_external("environment", intrinsics::environment);
        self.register_external("call", intrinsics::call);

        self.register_external("+", intrinsics::add);
        self.register_external("-", intrinsics::sub);
        self.register_external("*", intrinsics::mul);
        self.register_external("/", intrinsics::div);

        self.register_external(">", intrinsics::greater_than);
        self.register_external("<", intrinsics::less_than);
        self.register_external("=", intrinsics::eq);
        self.register_external("<=", intrinsics::greater_equal_than);
        self.register_external(">=", intrinsics::less_equal_than);

        self.register_external("cons?", intrinsics::is_cons);
        self.register_external("nil?", intrinsics::is_nil);
        self.register_external("vec?", intrinsics::is_vec);
        self.register_external("int?", intrinsics::is_int);
        self.register_external("atom?", intrinsics::is_atom);
        self.register_external("function?", intrinsics::is_function);
        self.register_external("err?", intrinsics::is_error);
        self.register_external("type-of", intrinsics::type_of);

        self.register_external("vec/get", intrinsics::vec_get);
        self.register_external("vec/len", intrinsics::vec_len);
        self.register_external("vec/push!", intrinsics::vec_push);
        self.register_external("vec/pop!", intrinsics::vec_pop);
        self.register_external("vec/set!", intrinsics::vec_set);
        self.register_external("vec", intrinsics::vec);

        self.register_external("hash-map", intrinsics::hash_map);
        self.register_external("hash-map/get", intrinsics::hash_map_get);
        self.register_external("hash-map/set!", intrinsics::hash_map_set);
        self.register_external("hash-map/keys", intrinsics::hash_map_keys);
        self.register_external("hash-map/vals", intrinsics::hash_map_vals);
        self.register_external("hash-map/len", intrinsics::hash_map_len);

        self.register_external("string/len", intrinsics::string_length);
        self.register_external("string/slice", intrinsics::string_slice);
        self.register_external("string/concat", intrinsics::string_concat);
        self.register_external("string/split", intrinsics::string_split);
        self.register_external("string/get", intrinsics::string_index);
        self.register_external("string/contains?", intrinsics::string_contains);

        self.register_external("err/message", intrinsics::err_message);
        self.register_external("err/print-stack", intrinsics::err_print_stack);

        self.register_external("to-string", intrinsics::to_string);
        self.register_external("to-int", intrinsics::to_int);
        self.register_external("to-atom", intrinsics::to_atom);

        self.register_external("try*", intrinsics::try_);
        self.register_external("throw", intrinsics::throw);

        self.register_external("ffi/open", intrinsics::ffi_open);
        self.register_external("ffi/get", intrinsics::ffi_get);
        self.register_external("ffi/apply", intrinsics::ffi_apply);
    }

    pub fn find(&self, id: &str) -> Result<Value, RuntimeError> {
        self.frames
            .last()
            .unwrap()
            .find(id)
            .ok_or_else(|| RuntimeError::UndefinedName(id.to_owned()))
            .or_else(|_| self.get_def(id).map(|x| x.value))
    }

    pub fn get_def(&self, id: &str) -> Result<Def, RuntimeError> {
        let Expr::HashMap(ref mut map) = self.global.clone().borrow_mut().kind else {
            return Err(RuntimeError::from(
                "attempted to get def of malformed value",
            ));
        };

        let value = map
            .get(id)
            .map(|x| x.1.clone())
            .ok_or_else(|| RuntimeError::UndefinedName(id.to_owned()))?
            .assert_vector()?;

        if value.len() >= 3 {
            let fst = value[0].assert_atom()?;
            let snd = value[1].assert_atom()?;
            let third = value[2].clone();
            Ok(Def {
                name: fst,
                value: third,
                is_macro: snd == "macro",
            })
        } else {
            Err(RuntimeError::from(
                "attempted to get def of malformed value",
            ))
        }
    }

    pub fn insert(&mut self, id: String, value: Value) {
        self.frames.last_mut().unwrap().insert(id, value);
    }

    pub fn insert_global(
        &mut self,
        id: String,
        value: Value,
        is_macro: bool,
    ) -> Result<(), RuntimeError> {
        let ref_ = self.global.clone().borrow_mut();
        let Expr::HashMap(ref mut map) = ref_.kind else {
            return Err(RuntimeError::from(
                "attempted to get def of malformed value",
            ));
        };

        map.insert(
            id.clone(),
            (
                Expr::Id(id.clone()).to_value(),
                Def {
                    name: id,
                    value,
                    is_macro,
                }
                .into(),
            ),
        );

        Ok(())
    }

    pub fn push(&mut self, name: Option<String>, catch: bool, location: Location) -> &mut Frame {
        self.frames.push(Frame::new(name, catch, location));
        self.frames.last_mut().unwrap()
    }

    pub fn push_from(&mut self, frame: Frame) -> &mut Frame {
        self.frames.push(frame);
        self.frames.last_mut().unwrap()
    }

    pub fn pop(&mut self) {
        self.frames.pop();
    }

    pub fn enable_catching(&mut self) {
        self.last_frame().catch = true;
    }

    pub fn disable_catching(&mut self) {
        self.last_frame().catch = false;
    }

    pub fn unwind(&mut self) -> Vec<Frame> {
        let mut frames = Vec::new();

        while let Some(frame) = self.frames.pop() {
            if frame.catch {
                self.frames.push(frame);
                break;
            }
            frames.push(frame);
        }

        frames
    }

    pub fn register_external(&mut self, name: &str, f: Prim) {
        let value = Expr::Function(Extern(f)).to_value();
        let _ = self.insert_global(name.to_string(), value, false);
    }
}
