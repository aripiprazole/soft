use std::borrow::Cow;

use super::*;

pub trait Tagged
where
    Self: Sized + Clone,
{
    const TAG: u64;
    fn tag(self) -> Ptr<Self> {
        let ptr = Box::leak(Box::new(self)) as *const _ as u64;
        Ptr::new(ptr | Self::TAG)
    }

    fn untag<'a>(ptr: Ptr<Self>) -> Cow<'a, Self> {
        Cow::Borrowed(unsafe { ((ptr.0 & MASK) as *const Self).as_ref().unwrap() })
    }
}

impl Tagged for Int {
    const TAG: u64 = INT;

    fn tag(self) -> Ptr<Self> {
        Ptr::new((self.0 << 3) | Self::TAG)
    }

    fn untag<'a>(ptr: Ptr<Self>) -> Cow<'a, Self> {
        Cow::Owned(Int(ptr.0 >> 3))
    }
}

impl Tagged for Symbol {
    const TAG: u64 = SYMBOL;

    fn tag(self) -> Ptr<Self> {
        Ptr::new(self.0 | Self::TAG)
    }

    fn untag<'a>(ptr: Ptr<Self>) -> Cow<'a, Self> {
        Cow::Owned(Symbol(ptr.0 & MASK))
    }
}

impl Tagged for Char {
    const TAG: u64 = CHAR;

    fn tag(self) -> Ptr<Self> {
        Ptr::new(((self.0 as u64) << 32) | Self::TAG)
    }

    fn untag<'a>(ptr: Ptr<Self>) -> Cow<'a, Self> {
        Cow::Owned(Char((ptr.0 >> 32) as u32))
    }
}

impl Tagged for Bool {
    const TAG: u64 = BOOL;

    fn tag(self) -> Ptr<Self> {
        match self {
            Bool::False => Ptr::new(Self::TAG),
            Bool::True => Ptr::new(0b1 << 5 | Self::TAG),
        }
    }

    fn untag<'a>(ptr: Ptr<Self>) -> Cow<'a, Self> {
        if ptr.0 & 0b1 << 5 == 0b1 << 5 {
            Cow::Owned(Bool::True)
        } else {
            Cow::Owned(Bool::False)
        }
    }
}

impl Tagged for Nil {
    const TAG: u64 = NIL;

    fn tag(self) -> Ptr<Self> {
        Ptr::new(Self::TAG)
    }

    fn untag<'a>(_: Ptr<Self>) -> Cow<'a, Self> {
        Cow::Owned(Nil)
    }
}

impl Tagged for Cons {
    const TAG: u64 = CONS;
}

impl Tagged for Vector {
    const TAG: u64 = VECTOR;
}

impl Tagged for Str {
    const TAG: u64 = STR;
}

impl Tagged for Closure {
    const TAG: u64 = CLOSURE;
}

impl<T: Tagged> From<Ptr<T>> for UnknownPtr {
    fn from(ptr: Ptr<T>) -> Self {
        Self(ptr.0)
    }
}

impl<T: Tagged> From<Ptr<T>> for Value {
    fn from(value: Ptr<T>) -> Self {
        Value(value.into())
    }
}

impl<T: Tagged> From<T> for Value {
    fn from(value: T) -> Self {
        Value(value.tag().into())
    }
}

impl<T: Tagged> Ptr<T> {
    pub fn untag(&self) -> T {
        Tagged::untag(self.clone()).into_owned()
    }
}

impl From<UnknownPtr> for FatPtr {
    fn from(value: UnknownPtr) -> Self {
        match value.0 & 0b111 {
            INT => FatPtr::Int(Ptr::new(value.0).untag()),
            CONS => FatPtr::Cons(Ptr::new(value.0 & MASK)),
            VECTOR => FatPtr::Vector(Ptr::new(value.0 & MASK)),
            STR => FatPtr::Str(Ptr::new(value.0 & MASK)),
            SYMBOL => FatPtr::Symbol(Ptr::new(value.0 & MASK)),
            CLOSURE => FatPtr::Closure(Ptr::new(value.0 & MASK)),
            PRIMITIVE => match value.0 & 0b11111 {
                CHAR => FatPtr::Char(Ptr::new(value.0).untag()),
                BOOL => FatPtr::Bool(Ptr::new(value.0).untag()),
                NIL => FatPtr::Nil,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
