pub use crate::runtime::{Value, ValueRef};

pub mod value {
    pub use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_new_num(n: u64) -> ValueRef {
        ValueRef::new_num(n)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_cons(head: ValueRef, tail: ValueRef) -> ValueRef {
        ValueRef::cons(head, tail)
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_nil() -> ValueRef {
        ValueRef::nil()
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_head(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_head: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(head, _) => *head,
            _ => panic!("prim__Value_head: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_tail(list: ValueRef) -> ValueRef {
        if list.is_num() {
            panic!("prim__Value_tail: expected list, got number");
        }

        match list.to_value() {
            Value::Cons(_, tail) => *tail,
            _ => panic!("prim__Value_tail: expected list, got {:?}", list),
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn prim__Value_is_true(value: ValueRef) -> bool {
        if value.is_num() {
            value.num() != 0
        } else {
            match value.to_value() {
                Value::Nil => false,
                _ => true,
            }
        }
    }
}
