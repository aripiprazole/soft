use std::ptr::NonNull;

use crate::ptr::*;

/// Abstract data types

pub enum Object {}
pub enum Number {}
pub enum Symbol {}
pub enum Pair {}
pub enum Function {}
pub enum Vector {}

pub trait AsTagged {
    type Item;
    fn as_tagged(typ: Self::Item) -> TaggedPtr;
}

impl AsTagged for Object {
    type Item = ObjectNode;
    fn as_tagged(object_ptr: Self::Item) -> TaggedPtr {
        TaggedPtr {
            object: NonNull::new(Box::leak(Box::new(object_ptr)) as *mut ObjectNode).unwrap(),
        }
    }
}

impl AsTagged for Number {
    type Item = isize;
    fn as_tagged(number: Self::Item) -> TaggedPtr {
        let mut result = TaggedPtr {
            number: number << 2,
        };
        unsafe { result.tag |= PtrTag::Number as usize };
        result
    }
}

impl AsTagged for Symbol {
    type Item = SymbolNode;
    fn as_tagged(symbol_ptr: Self::Item) -> TaggedPtr {
        let mut result = TaggedPtr {
            symbol: NonNull::new(Box::leak(Box::new(symbol_ptr)) as *mut SymbolNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Symbol as usize };
        result
    }
}

impl AsTagged for Pair {
    type Item = PairNode;
    fn as_tagged(pair_ptr: Self::Item) -> TaggedPtr {
        let mut result = TaggedPtr {
            pair: NonNull::new(Box::leak(Box::new(pair_ptr)) as *mut PairNode).unwrap(),
        };
        unsafe { result.tag |= PtrTag::Pair as usize };
        result
    }
}

impl AsTagged for Function {
    type Item = FunctionNode;

    #[inline(always)]
    fn as_tagged(function_ptr: Self::Item) -> TaggedPtr {
        Object::as_tagged(ObjectNode::Function(function_ptr))
    }
}

impl AsTagged for Vector {
    type Item = VectorNode;

    #[inline(always)]
    fn as_tagged(vector_ptr: Self::Item) -> TaggedPtr {
        Object::as_tagged(ObjectNode::Vector(vector_ptr))
    }
}
