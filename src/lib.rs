#![feature(box_patterns)]

pub mod repl;
pub mod runtime;
pub mod specialized;
pub mod util;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(pub soft);

#[cfg(test)]
mod tests {
    use super::*;
    use runtime::ValueRef;

    #[test]
    fn test_parser_num() {
        let result = soft::TermParser::new().parse("1").unwrap();

        assert_eq!(result, ValueRef::new_num(1));
    }

    #[test]
    fn test_parser_atom() {
        let result = soft::TermParser::new().parse("foo").unwrap();

        assert_eq!(result, ValueRef::atom("foo".to_string()));
    }

    #[test]
    fn test_parser_quote() {
        let result = soft::TermParser::new().parse("'foo").unwrap();

        assert_eq!(result, ValueRef::quote(ValueRef::atom("foo".to_string())));
    }

    #[test]
    fn test_parser_cons() {
        let result = soft::TermParser::new().parse("(1 2)").unwrap();

        assert_eq!(
            result,
            ValueRef::cons(
                ValueRef::new_num(1),
                ValueRef::cons(ValueRef::new_num(2), ValueRef::nil())
            )
        );
    }
}
