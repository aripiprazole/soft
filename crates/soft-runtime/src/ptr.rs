use std::{fmt::Display, marker::PhantomData, ptr::NonNull};

use crate::tag::AsTagged;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtrTag {
    Object = 0,
    Number = 1,
    Symbol = 2,
    Pair = 3,
}

pub struct VectorNode {
    pub size: usize,
    pub limit: usize,
    pub data: *mut TaggedPtr,
}

pub struct FunctionNode {
    pub data: *mut libc::c_void,
}

pub enum ObjectNode {
    Vector(VectorNode),
    Function(FunctionNode),
}

pub struct SymbolNode {
    data: &'static str,
}

pub struct PairNode {
    head: TaggedPtr,
    tail: TaggedPtr,
}

#[derive(Clone, Copy)]
pub union TaggedPtr {
    pub tag: usize,
    pub object: NonNull<ObjectNode>,
    pub number: isize,
    pub symbol: NonNull<SymbolNode>,
    pub pair: NonNull<PairNode>,
}

impl TaggedPtr {
    pub fn tag(&self) -> PtrTag {
        unsafe { std::mem::transmute(self.tag as u8 & 0b11) }
    }
}

impl TaggedPtr {
    pub fn new<T: AsTagged>(object_ptr: T::Item) -> Self {
        T::as_tagged(object_ptr)
    }

    pub fn number(number: isize) -> Self {
        let mut result = Self { number };
        unsafe { result.tag |= PtrTag::Number as usize };
        result
    }

    pub fn symbol(symbol_ptr: SymbolNode) -> Self {
        let mut result = Self {
            symbol: NonNull::new(Box::leak(Box::new(symbol_ptr)) as *mut SymbolNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Symbol as usize };
        result
    }

    pub fn pair(pair_ptr: PairNode) -> Self {
        let mut result = Self {
            pair: NonNull::new(Box::leak(Box::new(pair_ptr)) as *mut PairNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Pair as usize };
        result
    }
}

pub struct ScopedPtr<T>(TaggedPtr, PhantomData<T>);

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
            Value::Pair(p) => write!(
                f,
                "({} . {})",
                unsafe { Value::from(p.as_ref().head) },
                unsafe { Value::from(p.as_ref().tail) }
            ),
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

#[cfg(test)]
mod tests {
    use crate::ptr::*;
    use crate::tag::*;

    #[test]
    pub fn tagged_number() {
        let ptr = TaggedPtr::new::<Number>(322);
        assert_eq!(ptr.tag(), PtrTag::Number);
    }

    #[test]
    pub fn tagged_symbol() {
        let ptr = TaggedPtr::new::<Symbol>(SymbolNode { data: "cubes" });
        assert_eq!(ptr.tag(), PtrTag::Symbol);
    }

    #[test]
    pub fn tagged_pair() {
        let ptr = TaggedPtr::new::<Pair>(PairNode {
            head: TaggedPtr::new::<Number>(322),
            tail: TaggedPtr::new::<Number>(322),
        });
        assert_eq!(ptr.tag(), PtrTag::Pair);
    }

    #[test]
    pub fn tagged_vector() {
        let ptr = TaggedPtr::new::<Vector>(VectorNode {
            size: 3,
            limit: 3,
            data: Box::leak(Box::new([
                TaggedPtr::new::<Number>(322),
                TaggedPtr::new::<Number>(322),
                TaggedPtr::new::<Number>(322),
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
        let ptr = TaggedPtr::new::<Function>(FunctionNode {
            data: tagged_function as *mut libc::c_void,
        });
        assert_eq!(ptr.tag(), PtrTag::Object);

        match unsafe { ptr.object.as_ref() } {
            ObjectNode::Function(_) => {}
            _ => panic!("expected function"),
        }
    }
}
