use super::*;

pub trait Freeable where Self : crate::value::tagged::Tagged {
    const LAYOUT: Layout = Layout::new::<Self>();

    fn free(ptr: Ptr<Self>) {
        unsafe {
            dealloc(ptr.0 as *mut u8, Self::LAYOUT)
        }
    }
}

impl Freeable for Cons { }

impl Freeable for Vector { }

impl Freeable for Str { }

impl Freeable for Closure { }

impl<T: Freeable> Ptr<T> {
    pub fn free(self) {
        T::free(self)
    }
}