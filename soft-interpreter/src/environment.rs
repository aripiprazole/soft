//! An environment is the context in which an expression is evaluated. It contains a stack of call
//! frames that are used for each function and each [Frame] contains a stack of scopes that are used
//! for each block.

use im_rc::HashMap;

use crate::{
    intrinsics,
    value::{ExprKind, Function::Extern, Location, Prim, Value},
};

/// A frame is an abstraction for a function call space that contains a lot of information about
/// the lexical scope of the function.
#[derive(Debug, Clone)]
pub struct Frame {
    pub name: Option<String>,
    pub located_at: Location,
    catch: bool,
    stack: im_rc::Vector<im_rc::HashMap<String, Value>>,
}

impl Frame {
    pub fn new(name: Option<String>, catch: bool, located_at: Location) -> Self {
        Self {
            name,
            catch,
            stack: im_rc::vector![im_rc::HashMap::new()],
            located_at,
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

/// An environment is the context in which an expression is evaluated. It contains a stack of calls
/// and the global environment.
pub struct Environment {
    frames: Vec<Frame>,
    global: HashMap<String, Value>,
}

impl Environment {
    pub fn new(file: Option<String>) -> Self {
        Self {
            frames: vec![Frame::new(
                None,
                false,
                Location {
                    line: 1,
                    column: 0,
                    file,
                },
            )],
            global: HashMap::new(),
        }
    }

    pub fn last_frame(&self) -> &Frame {
        self.frames.last().unwrap()
    }

    pub fn register_intrinsics(&mut self) {
        self.register_external("fn*", intrinsics::lambda);
        self.register_external("set*", intrinsics::set);
        self.register_external("if", intrinsics::if_);
        self.register_external("<", intrinsics::less_than);
        self.register_external("+", intrinsics::add);
        self.register_external("-", intrinsics::sub);
    }

    pub fn find(&self, id: &str) -> Option<Value> {
        self.frames
            .last()
            .unwrap()
            .find(id)
            .or_else(|| self.global.get(id).cloned())
    }

    pub fn insert(&mut self, id: String, value: Value) {
        self.frames.last_mut().unwrap().insert(id, value);
    }

    pub fn insert_global(&mut self, id: String, value: Value) {
        self.global.insert(id, value);
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
        let value = ExprKind::Function(Extern(f)).to_value();
        self.global.insert(name.to_string(), value);
    }
}
