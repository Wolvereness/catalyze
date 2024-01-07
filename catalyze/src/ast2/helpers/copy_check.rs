use crate::ast2::helpers::Abstraction;

/// Makes sure that all of the node constructions don't need Drop.
#[allow(unused)]
struct CopyCheck;
impl<'a> Abstraction<'a> for CopyCheck {
    type Inner<I: 'a> = I;
}

impl Clone for super::super::Node<'_, CopyCheck> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for super::super::Node<'_, CopyCheck> {}
