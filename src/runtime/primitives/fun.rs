use super::*;

#[no_mangle]
pub extern "C" fn prim__fn_addr(value: ValueRef) -> AnyPtr {
    if value.is_num() {
        panic!("prim__fn_addr: expected pointer, got number");
    }

    match value.to_value() {
        Value::Function(_, ptr) => *ptr,
        _ => panic!("prim__fn_addr: expected pointer, got {value:?}"),
    }
}

#[no_mangle]
pub extern "C" fn prim__check_arity(value: ValueRef, args: u64) -> AnyPtr {
    if value.is_num() {
        panic!("prim__fn_check_arity: expected pointer, got number");
    }

    match value.to_value() {
        Value::Function(arity, ptr) => {
            if (*arity as u64) != args {
                panic!("prim__fn_check_arity: expected arity {value}, got {args}");
            }

            *ptr
        }
        _ => std::ptr::null_mut(),
    }
}
