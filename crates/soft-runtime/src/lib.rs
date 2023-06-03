use std::fmt::Debug;

#[repr(C)]
pub enum FatPtr {
    U61(u64),
}

#[no_mangle]
pub fn new_u64(value: u64) -> FatPtr {
    FatPtr::U61(value)
}

impl Debug for FatPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U61(value) => write!(f, "U61({value})"),
        }
    }
}
