//! This module creates a notion of pointer and tagged-pointer so we can use it to pass objects
//! around in the language. The main structure here is the [TaggedPtr].
use std::{marker::PhantomData, ptr::NonNull};

use crate::allocator::Allocator;

use self::sealed::{Complete, Taggable};

// Constants to masking of tagged pointers.

pub const POINTER_MASK: u64 = ((1 << 61) - 1) << 3;

pub const TAG_MASK: u64 = 0b111;

pub const BIG_TAG_MASK: u64 = 0b11111;

/// This is the 3 bits tag for a tagged pointer of the language. This means the language only runs
/// on 64 bit machines because 32 bit machines supports only 2 bits of tag.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    /// 61-bit number.
    Integer = 0b000,

    /// A common LISP cons-cell that contains two values.
    Pair = 0b001,

    /// A contiguous segment of memory that has a size and is immutable.
    Vector = 0b010,

    /// A utf-8 non indexable string.
    String = 0b011,

    /// A O(1) value that is similar to a string but it only compares a hash of it and it`s length
    Symbol = 0b100,

    /// The address of a native function or a interpreted version of it
    Function = 0b101,

    /// A tag for items that has less information like Chars, Booleans and Nil
    LessInfo = 0b110,

    /// A mutable reference for something.
    Ref = 0b111,

    /// A 59-bit char literal
    Char = 0b01_110,

    /// A couple of booleans or a single one. It only depends on how the person want to represent
    /// somethings.
    Boolean = 0b10_110,

    /// An empty value nil value.
    Nil = 0b11_110,
}

// A 61-bit number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct U61(u64);

impl std::ops::Add<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn add(self, rhs: U61) -> Self::Output {
        U61(self.0 + rhs.0)
    }
}

impl std::ops::Sub<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn sub(self, rhs: U61) -> Self::Output {
        U61(self.0 - rhs.0)
    }
}

impl std::ops::Mul<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn mul(self, rhs: U61) -> Self::Output {
        U61(self.0 * rhs.0)
    }
}

impl std::ops::Rem<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn rem(self, rhs: U61) -> Self::Output {
        U61(self.0 % rhs.0)
    }
}

impl std::ops::Shl<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn shl(self, rhs: U61) -> Self::Output {
        U61(self.0 << rhs.0)
    }
}

impl std::ops::Shr<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn shr(self, rhs: U61) -> Self::Output {
        U61(self.0 >> rhs.0)
    }
}

impl std::ops::BitAnd<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn bitand(self, rhs: U61) -> Self::Output {
        U61(self.0 & rhs.0)
    }
}

impl std::ops::BitOr<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn bitor(self, rhs: U61) -> Self::Output {
        U61(self.0 | rhs.0)
    }
}

impl std::ops::BitXor<U61> for U61 {
    type Output = U61;

    #[inline(always)]
    fn bitxor(self, rhs: U61) -> Self::Output {
        U61(self.0 ^ rhs.0)
    }
}

impl std::fmt::Display for U61 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}u61", self.0 >> 3)
    }
}

impl From<u64> for U61 {
    #[inline(always)]
    fn from(value: u64) -> Self {
        U61(value << 3)
    }
}

impl Taggable for U61 {
    const TAG: Tag = Tag::Integer;
}

// A 64-bit pointer with a 3-bit tag. It`s used to be space-efficient inside the runtime.
pub struct ScopedPtr<T>(TaggedPtr, PhantomData<T>);

impl From<U61> for ScopedPtr<U61> {
    #[inline(always)]
    fn from(value: U61) -> Self {
        ScopedPtr(TaggedPtr(value.0 | Tag::Integer as u64), Default::default())
    }
}

mod sealed {
    use super::Tag;

    /// Trait for objects that can turn into scoped pointers. It`s needs to make a better abstraction
    /// for numbers, booleans and other things turning into ScopedPtr safely.
    pub trait Scoped: Taggable {}

    /// Complete objects can return a complete tag instead of a "LessInfo" tag.
    pub trait Complete: From<u64> + Taggable {}

    /// For objects that can contain tags.
    pub trait Taggable {
        const TAG: Tag;
    }
}

impl ScopedPtr<U61> {
    /// Gets the 61-bit number inside of a tagged pointer.
    #[inline(always)]
    pub fn number(&self) -> U61 {
        // TODO: assert the integer tag is zero otherwise we need to change it to use the mask.
        U61(self.0 .0)
    }
}

impl<T: sealed::Scoped> ScopedPtr<T> {
    #[inline(always)]
    pub fn new(pointer: NonNull<T>) -> Self {
        ScopedPtr(TaggedPtr::new(pointer), Default::default())
    }

    /// Gets the pointer inside of a tagged pointer
    #[inline(always)]
    pub fn pointer(&self) -> &'static T {
        unsafe { self.0.pointer() }
    }

    /// Returns the tag of a pointer. All the TaggedPtr have a Tag.
    #[inline(always)]
    pub fn tag(&self) -> Tag {
        unsafe { self.0.unsafe_tag() }
    }
}

impl<T: sealed::Complete> ScopedPtr<T> {
    #[inline(always)]
    pub fn new_u59(&self, data: T) -> Self
    where
        T: Into<u64>,
    {
        ScopedPtr(TaggedPtr::new_u59::<T>(data), Default::default())
    }

    /// Gets the value of a complete ScopedPtr
    #[inline(always)]
    pub fn value(&self) -> T {
        unsafe { self.0.value() }
    }

    /// Returns a complete tag (including 5-bits).
    #[inline(always)]
    pub fn complete_tag(&self) -> Tag {
        unsafe { self.0.complete_tag() }
    }
}

#[derive(Debug)]
pub struct Pair {
    pub head: TaggedPtr,
    pub tail: TaggedPtr,
}

#[derive(Debug)]
pub struct Vector {
    pub data: Vec<TaggedPtr>,
}

#[derive(Debug)]
pub struct Symbol {
    pub str: String,
}

#[derive(Debug)]
pub struct Function {
    pub ptr: *mut libc::c_void,
    pub arity: u8,
    pub closure: Vec<TaggedPtr>,
}

#[derive(Debug)]
pub struct Ref {
    pub ptr: *mut libc::c_void,
}

#[derive(Debug)]
pub struct Bool(pub bool);

#[derive(Debug)]
pub struct Char(pub char);

impl From<u64> for Bool {
    fn from(value: u64) -> Self {
        Bool(value != 0)
    }
}

impl From<u64> for Char {
    fn from(value: u64) -> Self {
        unsafe { Char(char::from_u32_unchecked(value as u32)) }
    }
}

impl From<Bool> for u64 {
    fn from(value: Bool) -> Self {
        value.0 as u64
    }
}

impl From<Char> for u64 {
    fn from(value: Char) -> Self {
        value.0 as u64
    }
}

impl sealed::Taggable for Bool {
    const TAG: Tag = Tag::Boolean;
}

impl sealed::Taggable for Pair {
    const TAG: Tag = Tag::Pair;
}

impl sealed::Taggable for Vector {
    const TAG: Tag = Tag::Vector;
}

impl sealed::Taggable for Symbol {
    const TAG: Tag = Tag::Symbol;
}

impl sealed::Taggable for Function {
    const TAG: Tag = Tag::Function;
}

impl sealed::Taggable for Ref {
    const TAG: Tag = Tag::Ref;
}

impl Taggable for String {
    const TAG: Tag = Tag::String;
}

impl Taggable for Char {
    const TAG: Tag = Tag::Char;
}

impl sealed::Scoped for Pair {}

impl sealed::Scoped for Vector {}

impl sealed::Scoped for Symbol {}

impl sealed::Scoped for Function {}

impl sealed::Scoped for Ref {}

impl sealed::Scoped for String {}

impl sealed::Complete for Bool {}

impl sealed::Complete for Char {}

// A sealed trait that is used internally.
pub trait Scoped: sealed::Scoped {}

// TODO: It probably leaks the interface TwT
impl<T: sealed::Scoped> Scoped for T {}

/// Existential ScopedPtr, it removes the tag from it but can be turned safely into one of the
/// variants.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TaggedPtr(u64);

impl<T> From<ScopedPtr<T>> for TaggedPtr {
    #[inline(always)]
    fn from(value: ScopedPtr<T>) -> Self {
        value.0
    }
}

use std::fmt::Debug;

impl Debug for TaggedPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &(*self).into() {
            FatPtr::Integer(arg0) => f.debug_tuple("TaggedPtr::Integer").field(arg0).finish(),
            FatPtr::Pair(arg0) => f.debug_tuple("TaggedPtr::Pair").field(arg0).finish(),
            FatPtr::Vector(arg0) => f.debug_tuple("TaggedPtr::Vector").field(arg0).finish(),
            FatPtr::String(arg0) => f.debug_tuple("TaggedPtr::String").field(arg0).finish(),
            FatPtr::Symbol(arg0) => f.debug_tuple("TaggedPtr::Symbol").field(arg0).finish(),
            FatPtr::Function(arg0) => f.debug_tuple("TaggedPtr::Function").field(arg0).finish(),
            FatPtr::Ref(arg0) => f.debug_tuple("TaggedPtr::Ref").field(arg0).finish(),
            FatPtr::Char(arg0) => f.debug_tuple("TaggedPtr::Char").field(arg0).finish(),
            FatPtr::Boolean(arg0) => f.debug_tuple("TaggedPtr::Boolean").field(arg0).finish(),
            FatPtr::Nil => write!(f, "TaggedPtr::Nil"),
        }
    }
}

impl TaggedPtr {
    #[inline(always)]
    pub fn new<T: sealed::Scoped>(data: NonNull<T>) -> TaggedPtr {
        TaggedPtr(data.as_ptr() as u64 | T::TAG as u64)
    }

    #[inline(always)]
    pub fn alloc<T: Scoped>(data: T, allocator: impl Allocator) -> TaggedPtr {
        TaggedPtr::new(allocator.alloc(data))
    }

    pub fn new_u59<T: sealed::Complete + Into<u64>>(data: T) -> TaggedPtr {
        TaggedPtr(data.into() << 5 | T::TAG as u64)
    }

    #[inline(always)]
    pub fn new_number<T: Into<U61>>(data: T) -> TaggedPtr {
        TaggedPtr(data.into().0 | Tag::Integer as u64)
    }

    /// Creates a ScopedPtr out of thin air!
    ///
    /// # Safety
    /// It`s risky because you can make any strange TaggedPtr into a even more odd
    /// ScopedPtr<T> where T is something strange too.
    #[inline(always)]
    pub unsafe fn from_any<T: Taggable>(self) -> ScopedPtr<T> {
        ScopedPtr(self, Default::default())
    }

    /// Gets the pointer inside of a tagged pointer
    ///
    /// # Safety
    /// It`s risky because we don`t know the tag of the pointer.
    #[inline(always)]
    pub unsafe fn pointer<T: sealed::Scoped>(&self) -> &'static T {
        unsafe { ((self.0 & POINTER_MASK) as *mut T).as_ref().unwrap() }
    }

    /// Returns the tag of a pointer. All the TaggedPtr have a Tag.
    ///
    /// # Safety
    /// It`s bad because we only check 3 bits instead of all of 5 of them that are used.
    #[inline(always)]
    pub unsafe fn unsafe_tag(&self) -> Tag {
        unsafe { std::mem::transmute((self.0 & TAG_MASK) as u8) }
    }

    /// Gets the value of a complete ScopedPtr
    ///
    /// # Safety
    /// Risky because we don`t know the tag.
    #[inline(always)]
    pub unsafe fn value<T: Complete>(&self) -> T {
        T::from(self.0 >> 5)
    }

    /// Returns a complete tag (including 5-bits).
    ///
    /// # Safety
    /// Risky because we don`t know the tag at compile time.
    #[inline(always)]
    pub unsafe fn complete_tag(&self) -> Tag {
        unsafe { std::mem::transmute((self.0 & BIG_TAG_MASK) as u8) }
    }

    pub fn tag(&self) -> Tag {
        unsafe {
            match self.unsafe_tag() {
                Tag::LessInfo => self.complete_tag(),
                tag => tag,
            }
        }
    }

    pub fn convert<T: Taggable>(&self) -> Option<ScopedPtr<T>> {
        if self.tag() == T::TAG {
            Some(unsafe { self.from_any() })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn assert<T: Taggable>(&self) -> ScopedPtr<T> {
        self.convert().unwrap_or_else(|| {
            panic!(
                "[error] cannot convert '{:?}' to '{:?}'",
                self.tag(),
                T::TAG
            )
        })
    }
}

#[derive(Debug)]
pub enum FatPtr {
    Integer(U61),
    Pair(&'static Pair),
    Vector(&'static Vector),
    String(&'static str),
    Symbol(&'static Symbol),
    Function(&'static Function),
    Ref(&'static Ref),
    Char(char),
    Boolean(bool),
    Nil,
}

impl From<TaggedPtr> for FatPtr {
    fn from(value: TaggedPtr) -> Self {
        unsafe {
            match value.tag() {
                Tag::Integer => FatPtr::Integer(TaggedPtr::from_any::<U61>(value).number()),
                Tag::Pair => FatPtr::Pair(TaggedPtr::from_any::<Pair>(value).pointer()),
                Tag::Vector => FatPtr::Vector(TaggedPtr::from_any::<Vector>(value).pointer()),
                Tag::String => FatPtr::String(TaggedPtr::from_any::<String>(value).pointer()),
                Tag::Symbol => FatPtr::Symbol(TaggedPtr::from_any::<Symbol>(value).pointer()),
                Tag::Function => FatPtr::Function(TaggedPtr::from_any::<Function>(value).pointer()),
                Tag::Ref => FatPtr::Ref(TaggedPtr::from_any::<Ref>(value).pointer()),
                Tag::Char => FatPtr::Char(char::from_u32_unchecked(
                    TaggedPtr::from_any::<U61>(value).number().0 as u32,
                )),
                Tag::Boolean => FatPtr::Boolean(TaggedPtr::from_any::<U61>(value).number().0 != 0),
                Tag::Nil => FatPtr::Nil,
                Tag::LessInfo => unreachable!("cannot go this branch because it`s impossible"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bool, U61};
    use super::{FatPtr, Function, Pair, Ref, Symbol, Tag, TaggedPtr, Vector};

    type Test = (TaggedPtr, Tag, fn(FatPtr) -> bool);

    #[test]
    fn test_if_tags_are_ok() {
        let number = TaggedPtr::new_number(123);

        assert_eq!(
            "123u61",
            number.convert::<U61>().unwrap().number().to_string()
        );

        let pair = TaggedPtr::alloc(Pair {
            head: number,
            tail: number,
        }, crate::allocator::ALLOCATOR);

        let vec = TaggedPtr::alloc(Vector {
            data: vec![pair, number, pair],
        }, crate::allocator::ALLOCATOR);

        let symbol = TaggedPtr::alloc(Symbol {
            str: "atapo".to_string(),
        }, crate::allocator::ALLOCATOR);

        let function = TaggedPtr::alloc(Function {
            ptr: test_if_tags_are_ok as *mut libc::c_void,
            arity: 0,
            closure: vec![],
        }, crate::allocator::ALLOCATOR);

        let reference = TaggedPtr::alloc(Ref {
            ptr: (&function) as *const TaggedPtr as *mut libc::c_void,
        }, crate::allocator::ALLOCATOR);

        let bool = TaggedPtr::new_u59(Bool(false));

        let tests: [Test; 7] = [
            (function, Tag::Function, |x| {
                matches!(x, FatPtr::Function(_))
            }),
            (number, Tag::Integer, |x| matches!(x, FatPtr::Integer(_))),
            (pair, Tag::Pair, |x| matches!(x, FatPtr::Pair(_))),
            (vec, Tag::Vector, |x| matches!(x, FatPtr::Vector(_))),
            (symbol, Tag::Symbol, |x| matches!(x, FatPtr::Symbol(_))),
            (reference, Tag::Ref, |x| matches!(x, FatPtr::Ref(_))),
            (bool, Tag::Boolean, |x| matches!(x, FatPtr::Boolean(_))),
        ];

        for (tagged, tag, fat_test) in tests {
            assert_eq!(
                tagged.tag(),
                tag,
                "expected {:?} but got {:?}",
                tag,
                tagged.tag()
            );

            let fat = FatPtr::from(tagged);

            assert!(
                fat_test(fat),
                "the test with {:?} does not match it`s fatptr",
                tag
            );
        }
    }
}
