pub use super::*;

#[no_mangle]
pub extern "C" fn prim__Value_new_num(n: u64) -> ValueRef {
    ValueRef::new_num(n)
}

#[no_mangle]
pub extern "C" fn prim__Value_cons(head: ValueRef, tail: ValueRef) -> ValueRef {
    ValueRef::cons(head, tail)
}

#[no_mangle]
pub extern "C" fn prim__Value_nil() -> ValueRef {
    ValueRef::nil()
}

#[no_mangle]
pub extern "C" fn prim__Value_head(list: ValueRef) -> ValueRef {
    if list.is_num() {
        panic!("prim__Value_head: expected list, got number");
    }

    match list.to_value() {
        Value::Cons(head, _) => *head,
        _ => panic!("prim__Value_head: expected list, got {:?}", list),
    }
}

#[no_mangle]
pub extern "C" fn prim__Value_tail(list: ValueRef) -> ValueRef {
    if list.is_num() {
        panic!("prim__Value_tail: expected list, got number");
    }

    match list.to_value() {
        Value::Cons(_, tail) => *tail,
        _ => panic!("prim__Value_tail: expected list, got {:?}", list),
    }
}

#[no_mangle]
pub extern "C" fn prim__Value_is_true(value: ValueRef) -> bool {
    if value.is_num() {
        value.num() != 0
    } else {
        !matches!(value.to_value(), Value::Nil)
    }
}

#[no_mangle]
pub extern "C" fn prim__Value_function(arity: u64, function_ptr: AnyPtr) -> ValueRef {
    ValueRef::function(arity as _, function_ptr)
}

#[no_mangle]
pub extern "C" fn prim__Value_gep(ptr: ValueRef, index: u64) -> ValueRef {
    if ptr.is_num() {
        panic!("prim__Value_gep: expected pointer, got number");
    }

    match ptr.to_value() {
        Value::Vec(ref items, _) => unsafe { items.add(index as _).read() },
        _ => panic!("prim__Value_gep: expected pointer, got {ptr:?}"),
    }
}
