use crate::ast2::nodes::{File, Message};

pub struct MethodData<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub file: &'a str,
    pub name: &'a str,
}

impl MethodData<'_> {
    pub unsafe fn populate_into(&self, ast: &mut super::super::AstHydration, ptr: *mut Method) -> Result<(), super::super::Error> {
        populate!(ast, (*ptr = self)[
            input: Message,
            output: Message,
            name: str,
            file: File,
        ]);
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct Method<'a> {
    pub input: &'a Message,
    pub output: &'a Message,
    pub file: &'a File,
    pub name: &'a str,
}
