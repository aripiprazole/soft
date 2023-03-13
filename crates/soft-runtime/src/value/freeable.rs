use super::*;

pub trait Freeable
where
    Self: crate::value::tagged::Tagged,
{
    const LAYOUT: Layout = Layout::new::<Self>();

    fn free(ptr: Ptr<Self>) {
        unsafe { dealloc(ptr.0 as *mut u8, Self::LAYOUT) }
    }
}

impl Freeable for Cons {}

impl Freeable for Vector {
    fn free(ptr: Ptr<Self>) {
        let res = ptr.untag();
        res.free();
        unsafe { dealloc(ptr.0 as *mut u8, Self::LAYOUT) }
    }
}

impl Freeable for Str {
    fn free(ptr: Ptr<Self>) {
        let res = ptr.untag();
        res.free();
        unsafe { dealloc(ptr.0 as *mut u8, Self::LAYOUT) }
    }
}

impl Freeable for Closure {
    fn free(ptr: Ptr<Self>) {
        let res = ptr.untag();
        res.env.free();
        unsafe { dealloc(ptr.0 as *mut u8, Self::LAYOUT) }
    }
}

impl<T: Freeable> Ptr<T> {
    pub fn free(self) {
        T::free(self)
    }
}

impl Vector {
    pub fn free(&self) {
        unsafe {
            dealloc(
                self.data as _,
                Layout::array::<Value>(self.size as usize).unwrap(),
            )
        }
    }
}

impl Str {
    pub fn free(&self) {
        let size = self.0.as_bytes().len();
        unsafe { dealloc(self.0.as_ptr() as _, Layout::array::<u8>(size).unwrap()) }
    }
}

impl Closure {
    pub fn free(&self) {
        self.env.free();
    }
}

impl Value {
    pub fn free(self) {
        match self.classify() {
            FatPtr::Cons(ptr) => ptr.free(),
            FatPtr::Vector(ptr) => ptr.free(),
            FatPtr::Str(ptr) => ptr.free(),
            FatPtr::Closure(ptr) => ptr.free(),
            _ => todo!(),
        }
    }
}
