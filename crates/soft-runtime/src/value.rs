//! This module describes the values that the soft runtime
//! manipulates. [Value] uses a pointer tagging scheme as each
//! pointer is 8 byte aligned in a 64 bit architecture.

use std::fmt::Display;

/// A pointer to a function.
pub struct FunPointer(*mut libc::c_void);

/// The fundamental 61 bit integer type for fast arithmetic
/// inside the soft-runtime
pub struct Int61(u64);

impl Int61 {
    pub fn new(data: u64) -> Self {
        Self(data << 3)
    }
}

impl Display for Int61 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0 >> 3)
    }
}

impl From<Int61> for Value {
    fn from(value: Int61) -> Self {
        Value(value.0)
    }
}

/// Struct that represents a `cons cell`.
pub struct Pair {
    pub fst: Value,
    pub snd: Value,
}

impl Pair {
    pub fn new(fst: Value, snd: Value) -> Pair {
        Pair { fst, snd }
    }
}

impl Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} . {})", self.fst, self.snd)
    }
}

impl From<Pair> for Value {
    fn from(value: Pair) -> Self {
        Value((Box::leak(Box::new(value)) as *const _ as u64) | (Tag::Pair as u64))
    }
}

/// A continuous vector that cannot grow in size.
pub struct Vector {
    data: *mut Value,
    size: u64,
}

impl Vector {
    pub fn new(size: usize) -> Vector {
        use std::alloc::{self, Layout};

        let new_layout = Layout::array::<Value>(size).unwrap();

        Vector {
            data: unsafe { alloc::alloc(new_layout) as *mut Value },
            size: size as u64,
        }
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
}

impl From<Vector> for Value {
    fn from(value: Vector) -> Self {
        Value((Box::leak(Box::new(value)) as *const _ as u64) | (Tag::Vector as u64))
    }
}

impl Display for Vector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        if self.size > 0 {
            for i in 0..self.size {
                write!(f, "{}", self.get(i))?;
            }
        }
        write!(f, "}}")
    }
}

/// A pair of a vector and a function pointer.
pub struct Closure {
    pub env: Vector,
    pub addr: FunPointer,
}

impl Closure {
    pub fn new(env: Vector, addr: FunPointer) -> Closure {
        Closure { env, addr }
    }
}

impl From<Closure> for Value {
    fn from(value: Closure) -> Self {
        Value((Box::leak(Box::new(value)) as *const _ as u64) | (Tag::Closure as u64))
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<closure>")
    }
}

/// A simple boolean.
pub enum Bool {
    False = 0,
    True,
}

impl From<Bool> for Value {
    fn from(value: Bool) -> Self {
        Value(((value as u64) << 6) | ((TagLessInfo::Nil as u64) << 3) | Tag::LessInfo as u64)
    }
}

impl Display for Bool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bool::False => write!(f, "true"),
            Bool::True => write!(f, "false"),
        }
    }
}

/// Null value.
pub struct Nil;

impl Display for Nil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nil")
    }
}

impl From<Nil> for Value {
    fn from(_: Nil) -> Self {
        Value(((TagLessInfo::Nil as u64) << 3) | Tag::LessInfo as u64)
    }
}

/// UTF-8 encoded char.
pub struct Char(u32);

impl From<Char> for Value {
    fn from(value: Char) -> Self {
        Value(((value.0 as u64) << 32) | ((TagLessInfo::Char as u64) << 3) | Tag::LessInfo as u64)
    }
}

/// A pointer tagged value. Each of the values that
/// it can assume are described in [Tag].
#[derive(Clone, Copy)]
pub struct Value(u64);

impl Value {
    pub fn get_tag(&self) -> Tag {
        unsafe { std::mem::transmute((self.0 & 0b111) as u8) }
    }

    #[inline]
    pub fn as_int(self) -> Int61 {
        Int61(self.0)
    }

    pub fn as_pair(self) -> &'static Pair {
        unsafe { ((self.0 & 0xfffffffffffffff8) as *mut Pair).as_ref().unwrap() }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_tag() {
            Tag::Integer => write!(f, "{}", self.as_int()),
            Tag::Pair => write!(f, "{}", self.as_pair()),
            Tag::Vector => todo!(),
            Tag::String => todo!(),
            Tag::Symbol => todo!(),
            Tag::Closure => todo!(),
            Tag::LessInfo => todo!(),
            Tag::Unused => todo!(),
        }
    }
}

pub enum Tag {
    Integer = 0,
    Pair = 1,
    Vector = 2,
    String = 3,
    Symbol = 4,
    Closure = 5,
    LessInfo = 6,
    Unused = 7
}

pub enum TagLessInfo {
    Char = 0,
    Boolean = 1,
    Nil = 2,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn value_works() {
        let result: Value = Pair::new(Int61::new(3).into(), Int61::new(10).into()).into();
        assert_eq!(result.to_string(), "(3 . 10)")
    }
}