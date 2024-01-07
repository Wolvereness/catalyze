use std::marker::PhantomData;
use std::mem::MaybeUninit;
use either::Either;

pub trait Abstraction<'a> {
    type Inner<I: 'a>;
}

pub struct MutPtr;
impl<'a> Abstraction<'a> for MutPtr {
    type Inner<I: 'a> = *mut I;
}

pub struct Uniq<'a>(PhantomData<&'a mut ()>);
impl<'a> Abstraction<'a> for Uniq<'a> {
    type Inner<I: 'a> = &'a mut I;
}

pub struct Shared<'a>(PhantomData<&'a ()>);
impl<'a> Abstraction<'a> for Shared<'a> {
    type Inner<I: 'a> = &'a I;
}

pub struct Lazy;
impl<'a> Abstraction<'a> for Lazy {
    type Inner<I: 'a> = Either<<MutPtr as Abstraction<'a>>::Inner<MaybeUninit<I>>, <MutPtr as Abstraction<'a>>::Inner<I>>;
}
