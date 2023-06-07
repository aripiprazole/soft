use crate::ptr::TaggedPtr;

#[no_mangle]
pub extern "C" fn prim__new_u61(value: u64) -> TaggedPtr {
    TaggedPtr::new_number(value)
}

#[no_mangle]
pub extern "C" fn prim__add_tagged(left: TaggedPtr, right: TaggedPtr) -> TaggedPtr {
    let num = left.assert().number();
    let num1 = right.assert().number();
    TaggedPtr::new_number(num + num1)
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
