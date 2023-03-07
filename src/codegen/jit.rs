use im::HashMap;

use crate::runtime::ValueRef;

#[derive(Clone)]
pub struct GlobalRef {
    pub addr: ValueRef,
}

impl GlobalRef {
    pub fn new(addr: ValueRef) -> Self {
        Self { addr }
    }
}

#[derive(Default, Clone)]
pub struct GlobalEnvironment {
    pub symbols: HashMap<String, GlobalRef>,
}
