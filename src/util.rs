use std::fmt::Display;

macro_rules! bool_enum {
    ($name:ident) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum $name {
            Yes,
            No,
        }
    };
}

pub(crate) use bool_enum;

pub enum Mode {
    Interperse,
    Before,
}

pub struct Spaced<'a, T>(pub Mode, pub &'static str, pub &'a [T])
where
    T: Display;

impl<'a, T> Display for Spaced<'a, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Spaced(Mode::Interperse, string, slice) => {
                if !slice.is_empty() {
                    write!(f, "{}", slice[0])?;
                    for element in &slice[1..] {
                        write!(f, "{string}{element}")?;
                    }
                }
            }
            Spaced(Mode::Before, string, slice) => {
                for element in slice.iter() {
                    write!(f, "{string}{element}")?;
                }
            }
        }
        Ok(())
    }
}
