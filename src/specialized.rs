use crate::util::bool_enum;

pub enum Term {
    Lam(Lifted, Vec<String>, Box<Term>),
    Let(Vec<(String, Term)>, Box<Term>),
    App(Box<Term>, Vec<Term>),
    Binop(Operator, Box<Term>, Box<Term>),
    Set(String, IsMacro, Box<Term>),
    HelperRef(u64),
    LocalRef(String),
    GlobalRef(String),
}

pub enum Operator {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %
    Eq,  // ==
    Neq, // !=
    Lt,  // <
    Gt,  // >
    Lte, // <=
    Gte, // >=
    And, // &&
    Or,  // ||
    Not, // !
}

bool_enum!(IsMacro);
bool_enum!(Lifted);
