// This module defines a trait called [Allocator] that is used as heap of the soft runtime. With
// that we have a global variable that is being currently used to easily manage things inside of
// the runtime instead of just having it as object.

use std::ptr::NonNull;

pub use crate::ptr::Scoped;

pub const ALLOCATOR: GlobalAllocator = GlobalAllocator {};

pub trait Allocator {
    fn alloc<T: Scoped>(&self, ata: T) -> NonNull<T>;
}

pub struct GlobalAllocator {}

impl Allocator for GlobalAllocator {
    fn alloc<T: Scoped>(&self, data: T) -> NonNull<T> {
        NonNull::new(Box::leak(Box::new(data))).unwrap()
    }
}

#[inline(always)]
pub fn alloc<T: Scoped>(data: T) -> NonNull<T> {
    ALLOCATOR.alloc(data)
}
