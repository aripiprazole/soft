macro_rules! cstr {
    ($s:ident) => {
        format!("{}\0", $s).as_ptr() as *const i8
    };
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
    () => {
        "\0".as_ptr() as *const i8
    };
}

macro_rules! bool_enum {
    ($name:ident) => {
        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        pub enum $name {
            Yes,
            No,
        }
    };
}

macro_rules! llvm_wrapper {
    ($n:ident, $target:ident, $print_fn:ident) => {
        pub struct $n(pub $target);

        impl std::ops::Deref for $n {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Debug for $n {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self}")
            }
        }

        impl std::fmt::Display for $n {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                unsafe {
                    let string = CStr::from_ptr($print_fn(self.0)).to_string_lossy();
                    write!(f, "{string}")
                }
            }
        }
    };
}

pub(crate) use bool_enum;
pub(crate) use cstr;
pub(crate) use llvm_wrapper;
