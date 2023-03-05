pub mod runtime;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub soft);

#[cfg(test)]
mod tests {
    use super::*;
    use runtime::{Value, ValueRef};

    #[test]
    fn test_parser_num() {
        let result = soft::TermParser::new().parse("1").unwrap();

        assert_eq!(result.to_string(), ValueRef::new_num(1).to_string());
    }

    #[test]
    fn test_parser_atom() {
        let result = soft::TermParser::new().parse("foo").unwrap();

        assert_eq!(
            result.to_string(),
            ValueRef::new(Value::Atom("foo".to_string())).to_string()
        );
    }

    #[test]
    fn test_parser_quote() {
        let result = soft::TermParser::new().parse("'foo").unwrap();

        assert_eq!(
            result.to_string(),
            ValueRef::new(Value::Quote(ValueRef::new(Value::Atom("foo".to_string())))).to_string()
        );
    }

    #[test]
    fn test_parser_cons() {
        let result = soft::TermParser::new().parse("(1 2)").unwrap();

        assert_eq!(
            result.to_string(),
            ValueRef::new(Value::Cons(
                ValueRef::new_num(1),
                ValueRef::new(Value::Cons(ValueRef::new_num(2), ValueRef::new(Value::Nil)))
            ))
            .to_string()
        );
    }
}
