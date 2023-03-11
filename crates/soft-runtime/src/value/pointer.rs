use super::*;
use crate::value::tagged::Tagged;

#[derive(Clone, Debug)]
pub struct FunPtr(*mut libc::c_void);

/// The fundamental 61 bit integer type for fast arithmetic
/// inside the soft-runtime
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub struct Int(pub(crate) u64);

/// Struct that represents a `cons cell`.
#[derive(Clone, Debug)]
pub struct Cons {
    /// Head of the cons cell.
    pub head: Value,
    /// Tail of the cons cell.
    pub tail: Value,
}

/// A continuous vector that cannot grow in size.
#[derive(Clone, Debug)]
pub struct Vector {
    pub(crate) data: *mut Value,
    pub(crate) size: u64,
}

/// Fixed size string.
#[derive(Clone, Debug)]
pub struct Str(pub(crate) &'static str);

/// Symbol hash.
#[derive(Clone, Debug)]
pub struct Symbol(pub(crate) u64);

/// A pair of a vector and a function pointer.
#[derive(Clone, Debug)]
pub struct Closure {
    /// Environment of the closure.
    /// This is a vector of values that are used
    /// as the environment of the closure.
    pub env: Vector,
    /// Function pointer.
    /// This is the address of the function that
    /// is called when the closure is called.
    /// The function pointer is a function that
    /// takes a vector of values and returns a value.
    pub addr: FunPtr,
}

/// UTF-8 encoded char.
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub struct Char(pub(crate) u32);

/// A simple boolean.
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum Bool {
    False,
    True,
}

/// Null value.
#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub struct Nil;

/// A pointer that is tagged but we don't have the value.
#[derive(Clone, Copy, Debug)]
pub struct UnknownPtr(pub(crate) u64);

/// A pointer that contains a tag.
#[derive(Clone, Copy, Debug)]
pub struct Ptr<T: Tagged>(pub(crate) u64, PhantomData<T>);

/// "Fat" tagged pointer (that uses 16 bytes) that is easier to work with in the
/// rust side.
#[derive(Clone, Debug)]
pub enum FatPtr {
    Int(Int),
    Symbol(Ptr<Symbol>),
    // Heap stuff
    Cons(Ptr<Cons>),
    Vector(Ptr<Vector>),
    Str(Ptr<Str>),
    Closure(Ptr<Closure>),
    // Primitive pointers
    Char(Char),
    Bool(Bool),
    Nil,
}

impl<T :Tagged> Ptr<T> {
    #[inline]
    pub fn new(ptr: u64) -> Self {
        Self(ptr, PhantomData)
    }
}

impl Int {
    pub fn new(num: u64) -> Self {
        Self(num)
    }
}

impl Vector {
    pub fn new(slice: Vec<Value>) -> Self {
        let size = slice.len() as u64;
        let data = Box::leak(slice.into_boxed_slice()).as_mut_ptr();

        Vector { data, size }
    }

    /// Sets a value inside the Vector. In debug mode it throws an error
    /// in case of out of bounds exceptions, in release it causes
    /// undefined behaviour.
    pub fn set(&mut self, index: u64, data: Value) {
        #[cfg(debug_assertions)]
        if index >= self.size {
            panic!("index {index} out of bounds of {}", self.size);
        }

        unsafe {
            *self.data.add(index as usize) = data;
        }
    }

    pub fn get(&self, index: u64) -> Value {
        #[cfg(debug_assertions)]
        if index >= self.size {
            panic!("index {index} out of bounds of {}", self.size);
        }

        unsafe { *self.data.add(index as usize).as_ref().unwrap() }
    }

    pub fn free(&self) {
        unsafe {
            dealloc(
                self.data as _,
                Layout::array::<Value>(self.size as usize).unwrap(),
            )
        }
    }
}

impl Str {
    pub fn new(str: String) -> Str {
        Str(Box::leak(str.into_boxed_str()))
    }

    pub fn free(&self) {
        let size = self.0.as_bytes().len();
        unsafe { dealloc(self.0.as_ptr() as _, Layout::array::<u8>(size).unwrap()) }
    }
}

impl Closure {
    pub fn new(env: Vector, addr: FunPtr) -> Closure {
        Closure { env, addr }
    }

    pub fn free(&self) {
        self.env.free();
    }
}

impl Symbol {
    pub fn new(sym: String) -> Symbol {
        Symbol(fxhash::hash64(&sym) & MASK)
    }
}

impl Char {
    pub fn new(char: char) -> Char {
        Char(char as u32)
    }
}

impl Cons {
    pub fn new(head: Value, tail: Value) -> Cons {
        Cons { head, tail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int() {
        let int = Int::new(42);
        let tagged = int.tag();
        let untagged = tagged.untag();

        assert_eq!(tagged.0, 42 << 3);
        assert_eq!(untagged.0, 42);
    }

    #[test]
    fn test_char() {
        let char = Char::new('a');
        let tagged = char.tag();
        let untagged = tagged.untag();

        assert_eq!(tagged.0 & MASK_PRIMITIVE, ('a' as u64) << 32);
        assert_eq!(untagged.0, 'a' as u32);
    }

    #[test]
    fn test_bool() {
        let bool = Bool::True;
        let tagged = bool.tag();
        let untagged = tagged.untag();

        assert_eq!(tagged.0 & MASK_PRIMITIVE, 0b1 << 5);
        assert_eq!(untagged, Bool::True);
    }

    #[test]
    fn test_nil() {
        let nil = Nil;
        let tagged = nil.tag();
        let untagged = tagged.untag();

        assert_eq!(tagged.0, NIL);
        assert_eq!(untagged, Nil);
    }

    #[test]
    fn test_symbol() {
        let sym = Symbol::new("abc".to_string());
        let tagged = sym.tag();
        let untagged = tagged.untag();

        assert_eq!(untagged.0, fxhash::hash64("abc") & MASK);
    }

    #[test]
    fn test_cons() {
        let va = Int::new(42).into();
        let vb = Nil.into();
        let cons1: Value = Cons::new(va, vb).into();
        let cons: Value = Cons::new(cons1, cons1).into();

        let r = cons.classify();
        assert_eq!(r.to_string(), "((42) 42)");

        cons.free();
        cons1.free();
    }

    #[test]
    fn test_vector() {
        let va = Int::new(42).into();
        let vb = Str::new("eu amo a gabii".to_string()).into();
        let cons1: Value = Cons::new(va, vb).into();
        let cons: Value = Cons::new(cons1, cons1).into();

        let vector: Value = Vector::new(vec![cons, cons]).into();

        let r = vector.classify();
        assert_eq!(r.to_string(), "[((42 . \"eu amo a gabii\") 42 . \"eu amo a gabii\") ((42 . \"eu amo a gabii\") 42 . \"eu amo a gabii\")]");

        vb.free();
        vector.free();
        cons.free();
        cons1.free();
    }

    #[test]
    fn test_str() {
        let str = Str::new("abc".to_string());
        let tagged = str.tag();
        let untagged = tagged.untag();

        assert_eq!(untagged.0, "abc".to_string());
    }
}