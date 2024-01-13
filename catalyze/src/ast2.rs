use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::{mem, ptr};
use std::sync::{Arc, Mutex};
use either::Either;
use snafu::Snafu;
use yoke::erased::ErasedArcCart;
use yoke::{Yoke, Yokeable};

mod helpers {
    mod abstraction;
    pub use abstraction::*;

    mod hydrating_name;
    pub use hydrating_name::*;

    mod copy_check;
}
use helpers::*;

mod nodes {
    mod package;
    pub use package::*;
    mod file;
    pub use file::*;
    mod message;
    pub use message::*;
    mod r#enum;
    pub use r#enum::*;
    mod enum_value;
    pub use enum_value::*;
    mod service;
    pub use service::*;
    mod method;
    pub use method::*;
    mod field;
    pub use field::*;
    mod oneof;
    pub use oneof::*;
    mod extension;
    pub use extension::*;
}
use nodes::*;

pub enum Node<'a, T: Abstraction<'a>> {
    Package(T::Inner<Package>),
    File(T::Inner<File>),
    Message(T::Inner<Message>),
    Enum(T::Inner<Enum>),
    EnumValue(T::Inner<EnumValue>),
    Service(T::Inner<Service>),
    Method(T::Inner<Method>),
    Field(T::Inner<Field>),
    Oneof(T::Inner<Oneof>),
    Extension(T::Inner<Extension>),
}

pub struct Ast<'a> {
    packages: &'a [&'a Package],
    files: &'a [&'a File],
    messages: &'a [&'a Message],
    enums: &'a [&'a Enum],
    enum_values: &'a [&'a EnumValue],
    services: &'a [&'a Service],
    methods: &'a [&'a Method],
    fields: &'a [&'a Field],
    oneofs: &'a [&'a Oneof],
    extensions: &'a [&'a Extension],
    nodes: HashMap<&'a str, Node<'a, Shared<'a>>>,
}

unsafe impl<'a> Yokeable<'a> for Ast<'static> {
    type Output = Ast<'a>;

    fn transform(&'a self) -> &'a Self::Output {
        unsafe { mem::transmute(self) } // FIXME - https://discord.com/channels/442252698964721669/443150878111694848/1193408926826377216
    }

    fn transform_owned(self) -> Self::Output {
        unsafe { mem::transmute(self) } // FIXME - https://discord.com/channels/442252698964721669/443150878111694848/1193408926826377216
    }

    unsafe fn make(from: Self::Output) -> Self {
        unsafe { mem::transmute(from) } // FIXME - https://discord.com/channels/442252698964721669/443150878111694848/1193408926826377216
    }

    fn transform_mut<F>(&'a mut self, f: F) where F: 'static + for<'b> FnOnce(&'b mut Self::Output) {
        unsafe { f(mem::transmute::<&mut Self, &mut Self::Output>(self)) }
    }
}


pub struct AstHydration {
    bump: bumpalo::Bump,
    nodes: HashMap<HydratingName, Node<'static, Lazy>>,
}

#[derive(Copy, Clone, Debug, Snafu)]
pub enum Error {
    Present,
    TypeMismatch,
    Incomplete,
}

macro_rules! nodefn {
    ($([$f1:ident, $f2:ident, $var:ident]($d:tt => $t:tt),)*) => {
        impl AstHydration {
            $(
                pub fn $f1(&mut self, name: &str, value: &$d) -> Result<(), Error> {
                    let ptr = match self.nodes.get_mut::<str>(name) {
                        None => {
                            let ptr: *mut MaybeUninit<$t> = self.bump.alloc(MaybeUninit::uninit());
                            let name = self.bump.alloc_str(name);
                            self.nodes.insert(HydratingName(name), Node::$t(Either::Right(ptr as *mut $t)));
                            ptr
                        },
                        Some(Node::$t(Either::Right(_))) =>
                            return Err(Error::Present),
                        Some(value @ Node::$t(_)) => {
                            let &mut Node::$t(Either::Left(ptr)) = value
                                else { unreachable!() };
                            *value = Node::$t(Either::Right(ptr as *mut $t));
                            ptr
                        },
                        _ =>
                            return Err(Error::TypeMismatch),
                    };
                    unsafe { value.populate_into(self, ptr as *mut $t) }
                }

                fn $f2(&mut self, name: &str) -> Result<*const MaybeUninit<$t>, Error> {
                    Ok(match self.nodes.get::<str>(name) {
                        None => {
                            let ptr: *mut MaybeUninit<$t> = self.bump.alloc(MaybeUninit::uninit());
                            let name = self.bump.alloc_str(name);
                            self.nodes.insert(HydratingName(name), Node::$t(Either::Left(ptr)));
                            ptr
                        },
                        Some(&Node::$t(Either::Left(ptr))) => ptr,
                        Some(&Node::$t(Either::Right(ptr))) => ptr as _,
                        _ =>
                            return Err(Error::TypeMismatch),
                    })
                }
            )*

            pub fn finish(self) -> Result<Yoke<Ast<'static>, ErasedArcCart>, Error> {
                $( let mut $var = 0; )*
                for node in self.nodes.values() {
                    match node {
                        $( Node::$t(Either::Left(_)) => return Err(Error::Incomplete), )*
                        $( Node::$t(Either::Right(_)) => $var += 1, )*
                    }
                }

                fn null_ptr<I, R>(_: I) -> *const R { ptr::null() }
                $(
                    let $var = self.bump.alloc_slice_fill_with($var, null_ptr);
                    let mut $var: (*const [*const $t], _) = ($var, $var);
                )*

                for node in self.nodes.values() {
                    match node {
                        $(
                            &Node::$t(Either::Right(ptr)) => {
                                let (slot, slice) = $var.1.split_first_mut().unwrap();
                                $var.1 = slice;
                                *slot = ptr
                            },
                        )*
                        _ => unreachable!(),
                    }
                }

                $( let $var = $var.0; )*

                let nodes = self.nodes;
                Ok(Yoke::attach_to_cart(Arc::new(Mutex::new(self.bump)), |_| {
                    Ast {
                        $( $var: unsafe { &*($var as *const [&$t]) }, )*
                        nodes: nodes.into_iter().map(
                            |(k, v)| (
                                unsafe { &*k.0 },
                                match v {
                                    $( Node::$t(Either::Right(ptr)) => Node::$t(unsafe { &*ptr }), )*
                                    _ => unreachable!(),
                                },
                            )
                        ).collect(),
                    }
                }))
            }
        }
    };
}

nodefn![
    [add_package, placeholder_package, packages](PackageData => Package),
    [add_file, placeholder_file, files](FileData => File),
    [add_message, placeholder_message, messages](MessageData => Message),
    [add_enum, placeholder_enum, enums](EnumData => Enum),
    [add_enum_value, placeholder_enum_value, enum_values](EnumValueData => EnumValue),
    [add_service, placeholder_service, services](ServiceData => Service),
    [add_method, placeholder_method, methods](MethodData => Method),
    [add_field, placeholder_field, fields](FieldData => Field),
    [add_oneof, placeholder_oneof, oneofs](OneofData => Oneof),
    [add_extension, placeholder_extension, extensions](ExtensionData => Extension),
];
