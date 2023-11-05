use crate::runtime::TaggedPtr;

#[derive(Default)]
pub struct Storage {}

#[repr(C)]
pub struct Environment(*mut Storage);

impl Default for Environment {
    fn default() -> Self {
        Self(Box::leak(Box::default()) as *mut Storage)
    }
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        unsafe {
            Self(Box::leak(Box::new(*self.0.clone())))
        }
    }
}

impl Environment {
    pub fn define(&mut self, name: String, expression: TaggedPtr) {}
}

pub mod stdlib {
    use super::*;

    pub unsafe extern "C" fn plus(environment: Environment) {}
}