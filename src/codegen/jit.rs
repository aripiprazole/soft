use im::HashMap;

use super::compile::SymbolRef;

#[derive(Default, Clone)]
pub struct GlobalEnvironment {
    pub symbols: HashMap<String, SymbolRef>,
}
