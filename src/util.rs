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
