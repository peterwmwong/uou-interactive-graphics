use super::bind::Binds;
use metal::{FunctionConstantValues, LibraryRef};

pub trait Function {
    const FUNCTION_NAME: &'static str;
    type Binds<'a>: Binds;

    #[inline(always)]
    fn get_function(&self, lib: &LibraryRef) -> metal::Function {
        lib.get_function(Self::FUNCTION_NAME, self.get_function_constants())
            .expect("Failed to get vertex function from library")
    }

    #[inline(always)]
    fn get_function_constants(&self) -> Option<FunctionConstantValues> {
        None
    }
}
