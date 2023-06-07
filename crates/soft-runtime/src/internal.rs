use std::ffi::CStr;

use crate::ptr::{Function, TaggedPtr};

#[no_mangle]
pub extern "C" fn prim__nil() -> TaggedPtr {
    TaggedPtr::new_number(0)
}

#[no_mangle]
pub extern "C" fn prim__function(value: u64) -> TaggedPtr {
    TaggedPtr::alloc(Function {
        ptr: value as *mut _,
        vec: vec![],
    })
}

#[no_mangle]
pub extern "C" fn prim__new_u61(value: u64) -> TaggedPtr {
    TaggedPtr::new_number(value)
}

#[no_mangle]
pub extern "C" fn prim__add_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs + rhs)
}

#[no_mangle]
pub extern "C" fn prim__sub_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs - rhs)
}

#[no_mangle]
pub extern "C" fn prim__mul_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs * rhs)
}

#[no_mangle]
pub extern "C" fn prim__mod_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs % rhs)
}

#[no_mangle]
pub extern "C" fn prim__shl_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs << rhs)
}

#[no_mangle]
pub extern "C" fn prim__shr_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs >> rhs)
}

#[no_mangle]
pub extern "C" fn prim__and_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs & rhs)
}

#[no_mangle]
pub extern "C" fn prim__or_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs | rhs)
}

#[no_mangle]
pub extern "C" fn prim__xor_tagged(lhs: TaggedPtr, rhs: TaggedPtr) -> TaggedPtr {
    let lhs = lhs.assert().number();
    let rhs = rhs.assert().number();
    TaggedPtr::new_number(lhs ^ rhs)
}

#[no_mangle]
pub extern "C" fn soft_panic(message: *mut libc::c_void) -> TaggedPtr {
    unsafe {
        let message = CStr::from_ptr(message as *mut _).to_string_lossy();
        panic!("[SOFT] Panic: {message}");
    }
}

#[cfg(test)]
mod tests {
    use crate::ptr::TaggedPtr;

    use super::prim__add_tagged;

    #[test]
    pub fn test_add() {
        let left = TaggedPtr::new_number(12);
        let right = TaggedPtr::new_number(3);
        let result = prim__add_tagged(left, right);
        assert_eq!("15u61", result.assert().number().to_string());
    }
}
