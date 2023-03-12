//! This module describes the values that the soft runtime
//! manipulates. [Value] uses a pointer tagging scheme as each
//! pointer is 8 byte aligned in a 64 bit architecture.

use std::alloc::{dealloc, Layout};
use std::marker::PhantomData;

use self::pointer::*;

pub mod display;
pub mod freeable;
pub mod pointer;
pub mod tagged;

pub const INT: u64 = 0b00_000;
pub const CONS: u64 = 0b00_001;
pub const VECTOR: u64 = 0b00_010;
pub const STR: u64 = 0b00_011;
pub const SYMBOL: u64 = 0b00_100;
pub const CLOSURE: u64 = 0b00_101;
pub const PRIMITIVE: u64 = 0b00_110;

pub const CHAR: u64 = 0b01_110;
pub const BOOL: u64 = 0b10_110;
pub const NIL: u64 = 0b11_110;

pub const MASK: u64 = 0xFFFFFFFFFFFFFFF8;

pub const MASK_PRIMITIVE: u64 = 0xFFFFFFFFFFFFFFE0;

#[derive(Clone, Copy, Debug)]
pub struct Value(pointer::UnknownPtr);

impl Value {
    pub fn classify(&self) -> FatPtr {
        self.0.into()
    }
}
