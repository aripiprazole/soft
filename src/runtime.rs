use std::fmt::Display;

pub struct ValueRef(u64);

impl ValueRef {
    pub fn is_num(&self) -> bool {
        self.0 & 1 == 1
    }

    pub fn num(&self) -> u64 {
        self.0 >> 1
    }

    pub fn new_value(value: Value) -> ValueRef {
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
            }
        }
    }
}

pub enum Value {
    Cons(ValueRef, ValueRef),
    Nil,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_ref() {
        let value = Value::Cons(ValueRef::new_num(1), ValueRef::new_num(2));

        let value_ref = ValueRef::new_value(value);

        assert_eq!(value_ref.to_string(), "(#1 #2)");
    }
}
