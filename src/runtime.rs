use std::fmt::{Debug, Display};

use crate::spaced::{Mode, Spaced};

use self::primitives::AnyPtr;

pub mod primitives;

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Cons(ValueRef, ValueRef),
    Atom(String),
    Closure(ValueRef, ValueRef),
    Function(u8, primitives::AnyPtr),
    Vec(usize, *mut ValueRef),
    Nil,
}

impl PartialEq for ValueRef {
    fn eq(&self, other: &Self) -> bool {
        if self.0 == other.0 {
            return true;
        }

        self.to_value() == other.to_value()
    }
}

impl Debug for ValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_num() {
            write!(f, "#{}", self.num())
        } else {
            let value = unsafe { std::mem::transmute::<u64, &Value>(self.0) };

            match value {
                Value::Nil => write!(f, "nil"),
                Value::Atom(value) => write!(f, "{value}"),
                Value::Cons(head, tail) => write!(f, "({head} {tail})"),
                Value::Closure(env, t) => write!(f, "<closure: {env} {t}>"),
                Value::Function(arity, _) => write!(f, "<function: {arity}>"),
                Value::Vec(len, items) => {
                    let elems = unsafe {
                        std::ptr::slice_from_raw_parts(*items, *len)
                            .as_ref()
                            .unwrap()
                    };

                    write!(f, "<vec{}>", Spaced(Mode::Before, " ", elems))
                }
            }
        }
    }
}
#[derive(Eq, Clone, Copy)]
#[repr(C)]
pub struct ValueRef(pub u64);

impl ValueRef {
    pub fn new(value: Value) -> ValueRef {
        let ptr = Box::leak(Box::new(value));
        ValueRef((ptr as *const Value as u64) | 1)
    }

    pub fn new_num(value: u64) -> ValueRef {
        ValueRef(value << 1)
    }

    pub fn to_value(&self) -> &Value {
        unsafe { std::mem::transmute::<u64, &Value>(self.0 & 0xFFFFFFFFFFFFFFFE) }
    }

    pub fn is_num(&self) -> bool {
        self.0 & 1 == 0
    }

    pub fn num(&self) -> u64 {
        self.0 >> 1
    }

    pub fn is_nil(&self) -> bool {
        self.maybe().map_or(false, |value| value == &Value::Nil)
    }

    pub fn maybe(&self) -> Option<&Value> {
        if self.is_num() {
            None
        } else {
            Some(self.to_value())
        }
    }

    pub fn nil() -> ValueRef {
        ValueRef::new(Value::Nil)
    }

    pub fn cons(head: ValueRef, tail: ValueRef) -> ValueRef {
        ValueRef::new(Value::Cons(head, tail))
    }

    pub fn quote(value: ValueRef) -> ValueRef {
        ValueRef::new(Value::Cons(
            ValueRef::new(Value::Atom("quote".to_string())),
            value,
        ))
    }

    pub fn atom(value: String) -> ValueRef {
        ValueRef::new(Value::Atom(value))
    }

    pub fn vec(len: usize, items: *mut ValueRef) -> ValueRef {
        ValueRef::new(Value::Vec(len, items))
    }

    pub fn closure(env: ValueRef, func: ValueRef) -> ValueRef {
        ValueRef::new(Value::Closure(env, func))
    }

    pub fn function(arity: u8, addr: AnyPtr) -> ValueRef {
        ValueRef::new(Value::Function(arity, addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_ref() {
        let value = Value::Cons(ValueRef::new_num(1), ValueRef::new_num(2));

        let value_ref = ValueRef::new(value);

        assert_eq!(value_ref.to_string(), "(#1 #2)");
    }

    #[test]
    fn test_value_size() {
        assert_eq!(std::mem::size_of::<Value>(), 32);
    }
}
