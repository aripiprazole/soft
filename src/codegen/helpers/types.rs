macro_rules! pointer_type {
    ($e:expr) => {
        unsafe { LLVMPointerType($e, 0) }
    };
}

macro_rules! function_type {
    ($return_type:expr, $arg_type:expr; $n:expr) => {{
        let mut args_types = vec![$arg_type; $n];
        unsafe {
            LLVMFunctionType($return_type, args_types.as_mut_ptr(), args_types.len() as u32, 0)
        }
    }};
    ($return_type:expr, $($args:expr),+ $(,)?) => {{
        let mut args_types = [$($args),+];
        unsafe {
            LLVMFunctionType($return_type, args_types.as_mut_ptr(), args_types.len() as u32, 0)
        }
    }};
}

pub(crate) use function_type;
pub(crate) use pointer_type;
