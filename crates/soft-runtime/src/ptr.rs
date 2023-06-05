//! This module creates a notion of pointer and tagged-pointer so we can use it to pass objects
//! around in the language. The main structure here is the [TaggedPtr].
use std::{fmt::Display, ptr::NonNull};

/// The first 2 bits of a [TaggedPtr] should be a [PtrTag] that says which is the type of the tagged
/// ptr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtrTag {
    Object = 0,
    Number = 1,
    Symbol = 2,
    Pair = 3,
}

/// A mutable dynamically sized array of [TaggedPtr]
pub struct VectorNode {
    pub size: usize,
    pub limit: usize,
    pub data: *mut TaggedPtr,
}

/// A function pointer node.
pub struct FunctionNode {
    pub data: *mut libc::c_void,
}

/// An object that is a "fat" structure but we don't care so much about that.
pub enum ObjectNode {
    Vector(VectorNode),
    Function(FunctionNode),
}

/// An unique identifier that is used as first-class values in the language.
pub struct SymbolNode {
    data: &'static str,
}

/// A cons-cell that is used to create lists and other data structures.
pub struct PairNode {
    head: TaggedPtr,
    tail: TaggedPtr,
}

/// A tagged pointer is an union type that can be used to represent any of the types that are used
/// inside the soft compiler. It is used to pass values around and to represent values in the stack.
///
/// # Safety
/// Don't use it directly, just by functions that are safe.
#[derive(Clone, Copy)]
pub union TaggedPtr {
    tag: usize,
    number: isize,
    object: NonNull<ObjectNode>,
    symbol: NonNull<SymbolNode>,
    pair: NonNull<PairNode>,
}

impl TaggedPtr {
    /// Gets the [PtrTag] (tag) of a [TaggedPtr].
    pub fn tag(&self) -> PtrTag {
        unsafe { std::mem::transmute(self.tag as u8 & 0b11) }
    }
}

impl TaggedPtr {
    pub fn new<T: IntoTagged>(object_ptr: T) -> Self {
        object_ptr.into_tagged()
    }

    /// Creates a new ObjectNode tagged pointer.
    pub fn object(obj: ObjectNode) -> Self {
        let mut result = Self {
            object: NonNull::new(Box::leak(Box::new(obj)) as *mut ObjectNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Object as usize };
        result
    }

    /// Creates a new number tagged pointer.
    pub fn number(number: isize) -> Self {
        let mut result = Self { number };
        unsafe { result.tag |= PtrTag::Number as usize };
        result
    }

    /// Creates a new symbol tagged pointer.
    pub fn symbol(symbol: SymbolNode) -> Self {
        let mut result = Self {
            symbol: NonNull::new(Box::leak(Box::new(symbol)) as *mut SymbolNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Symbol as usize };
        result
    }

    /// Creates a new pair tagged pointer.
    pub fn pair(pair: PairNode) -> Self {
        let mut result = Self {
            pair: NonNull::new(Box::leak(Box::new(pair)) as *mut PairNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Pair as usize };
        result
    }
}

/// A value is a first-class value in the language but here it's represented as a FatPtr so we can
/// pass it around a little bit easier.
pub enum Value {
    Number(isize),
    Symbol(NonNull<SymbolNode>),
    Pair(NonNull<PairNode>),
    Object(NonNull<ObjectNode>),
}

impl From<TaggedPtr> for Value {
    fn from(ptr: TaggedPtr) -> Self {
        unsafe {
            let obj = ptr.tag & 0xFFFFFFFFFFFFFFFC;
            match ptr.tag() {
                PtrTag::Object => Value::Object(NonNull::new(obj as *mut ObjectNode).unwrap()),
                PtrTag::Number => Value::Number(ptr.number >> 2),
                PtrTag::Symbol => Value::Symbol(NonNull::new(obj as *mut SymbolNode).unwrap()),
                PtrTag::Pair => Value::Pair(NonNull::new(obj as *mut PairNode).unwrap()),
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Symbol(s) => write!(f, ":{}", unsafe { s.as_ref().data }),
            Value::Pair(p) => unsafe {
                let head = Value::from(p.as_ref().head);
                let tail = Value::from(p.as_ref().tail);
                write!(f, "({} . {})", head, tail)
            },
            Value::Object(obj) => match unsafe { obj.as_ref() } {
                ObjectNode::Vector(v) => {
                    write!(f, "#(")?;
                    for i in 0..v.size {
                        write!(f, "{} ", unsafe { Value::from(*v.data.add(i)) })?;
                    }
                    write!(f, ")")
                }
                ObjectNode::Function(_) => write!(f, "#<function>"),
            },
        }
    }
}

/// A trait that is used to convert a type into a [TaggedPtr] without the concrete type. It's used
/// to make it a little bit clearer to the end user.
pub trait IntoTagged {
    fn into_tagged(self) -> TaggedPtr;
}

impl IntoTagged for ObjectNode {
    fn into_tagged(self) -> TaggedPtr {
        TaggedPtr {
            object: NonNull::new(Box::leak(Box::new(self)) as *mut ObjectNode).unwrap(),
        }
    }
}

impl IntoTagged for isize {
    fn into_tagged(self) -> TaggedPtr {
        let mut result = TaggedPtr { number: self << 2 };
        unsafe { result.tag |= PtrTag::Number as usize };
        result
    }
}

impl IntoTagged for SymbolNode {
    fn into_tagged(self) -> TaggedPtr {
        let mut result = TaggedPtr {
            symbol: NonNull::new(Box::leak(Box::new(self)) as *mut SymbolNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Symbol as usize };
        result
    }
}

impl IntoTagged for PairNode {
    fn into_tagged(self) -> TaggedPtr {
        let mut result = TaggedPtr {
            pair: NonNull::new(Box::leak(Box::new(self)) as *mut PairNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Pair as usize };
        result
    }
}

impl IntoTagged for FunctionNode {
    #[inline(always)]
    fn into_tagged(self) -> TaggedPtr {
        ObjectNode::Function(self).into_tagged()
    }
}

impl IntoTagged for VectorNode {
    #[inline(always)]
    fn into_tagged(self) -> TaggedPtr {
        ObjectNode::Vector(self).into_tagged()
    }
}

#[cfg(test)]
mod tests {
    use crate::ptr::*;

    #[test]
    pub fn tagged_number() {
        let ptr = TaggedPtr::new(322);
        assert_eq!(ptr.tag(), PtrTag::Number);
    }

    #[test]
    pub fn tagged_symbol() {
        let ptr = TaggedPtr::new(SymbolNode { data: "cubes" });
        assert_eq!(ptr.tag(), PtrTag::Symbol);
    }

    #[test]
    pub fn tagged_pair() {
        let ptr = TaggedPtr::new(PairNode {
            head: TaggedPtr::new(322),
            tail: TaggedPtr::new(322),
        });
        assert_eq!(ptr.tag(), PtrTag::Pair);
    }

    #[test]
    pub fn tagged_vector() {
        let ptr = TaggedPtr::new(VectorNode {
            size: 3,
            limit: 3,
            data: Box::leak(Box::new([
                TaggedPtr::new(322),
                TaggedPtr::new(322),
                TaggedPtr::new(322),
            ])) as *mut TaggedPtr,
        });
        assert_eq!(ptr.tag(), PtrTag::Object);

        match unsafe { ptr.object.as_ref() } {
            ObjectNode::Vector(v) => {
                assert_eq!(v.size, 3);
                assert_eq!(v.limit, 3);
            }
            _ => panic!("expected vector"),
        }
    }

    #[test]
    pub fn tagged_function() {
        let ptr = TaggedPtr::new(FunctionNode {
            data: tagged_function as *mut libc::c_void,
        });
        assert_eq!(ptr.tag(), PtrTag::Object);

        match unsafe { ptr.object.as_ref() } {
            ObjectNode::Function(_) => {}
            _ => panic!("expected function"),
        }
    }
}
