use std::fmt::Display;

pub struct ValueRef(u64);

impl ValueRef {
    pub fn is_num(&self) -> bool {
        self.0 & 1 == 1
    }

    pub fn num(&self) -> u64 {
        self.0 >> 1
    }

    pub fn nil() -> ValueRef {
        ValueRef::new(Value::Nil)
    }

    pub fn cons(head: ValueRef, tail: ValueRef) -> ValueRef {
        ValueRef::new(Value::Cons(head, tail))
    }

    pub fn quote(value: ValueRef) -> ValueRef {
        ValueRef::new(Value::Quote(value))
    }

    pub fn atom(value: String) -> ValueRef {
        ValueRef::new(Value::Atom(value))
    }

    pub fn new(value: Value) -> ValueRef {
        let ptr = Box::leak(Box::new(value));

        ValueRef(ptr as *const Value as u64)
    }

    pub fn new_num(value: u64) -> ValueRef {
        ValueRef(value << 1 | 1)
    }
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_num() {
            write!(f, "#{}", self.num())
        } else {
            let value = unsafe { std::mem::transmute::<u64, &Value>(self.0) };

            match value {
                Value::Cons(head, tail) => write!(f, "({} {})", head, tail),
                Value::Nil => write!(f, "nil"),
                Value::Quote(value) => write!(f, "'{}", value),
                Value::Atom(value) => write!(f, "{}", value),
            }
        }
    }
}

pub enum Value {
    Cons(ValueRef, ValueRef),
    Atom(String),
    Quote(ValueRef),
    Nil,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_ref() {
        let value = Value::Cons(ValueRef::new_num(1), ValueRef::new_num(2));

        let value_ref = ValueRef::new(value);

        assert_eq!(value_ref.to_string(), "(#1 #2)");
    }
}
