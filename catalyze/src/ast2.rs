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

macro_rules! hydration_helper {
    ($([$add_fn:ident, $placeholder_fn:ident, $field:ident]($type_data:tt => $type_name:tt),)*) => {
        impl AstHydration {
            $(
                pub fn $add_fn(&mut self, name: &str, value: &$type_data) -> Result<(), Error> {
                    let ptr = match self.nodes.get_mut::<str>(name) {
                        None => {
                            let ptr: *mut MaybeUninit<$type_name> = self.bump.alloc(MaybeUninit::uninit());
                            let name = self.bump.alloc_str(name);
                            self.nodes.insert(HydratingName(name), Node::$type_name(Either::Right(ptr as *mut $type_name)));
                            ptr
                        },
                        Some(Node::$type_name(Either::Right(_))) =>
                            return Err(Error::Present),
                        Some(value @ Node::$type_name(_)) => {
                            let &mut Node::$type_name(Either::Left(ptr)) = value
                                else { unreachable!() };
                            *value = Node::$type_name(Either::Right(ptr as *mut $type_name));
                            ptr
                        },
                        _ =>
                            return Err(Error::TypeMismatch),
                    };
                    unsafe { value.populate_into(self, ptr as *mut $type_name) }
                }

                fn $placeholder_fn(&mut self, name: &str) -> Result<*const MaybeUninit<$type_name>, Error> {
                    Ok(match self.nodes.get::<str>(name) {
                        None => {
                            let ptr: *mut MaybeUninit<$type_name> = self.bump.alloc(MaybeUninit::uninit());
                            let name = self.bump.alloc_str(name);
                            self.nodes.insert(HydratingName(name), Node::$type_name(Either::Left(ptr)));
                            ptr
                        },
                        Some(&Node::$type_name(Either::Left(ptr))) => ptr,
                        Some(&Node::$type_name(Either::Right(ptr))) => ptr as _,
                        _ =>
                            return Err(Error::TypeMismatch),
                    })
                }
            )*

            pub fn finish(self) -> Result<Yoke<Ast<'static>, ErasedArcCart>, Error> {
                $( let mut $field = 0; )*
                for node in self.nodes.values() {
                    match node {
                        $( Node::$type_name(Either::Left(_)) => return Err(Error::Incomplete), )*
                        $( Node::$type_name(Either::Right(_)) => $field += 1, )*
                    }
                }

                fn null_ptr<I, R>(_: I) -> *const R { ptr::null() }
                $(
                    let $field = self.bump.alloc_slice_fill_with($field, null_ptr);
                    let mut $field: (*const [*const $type_name], _) = ($field, $field);
                )*

                for node in self.nodes.values() {
                    match node {
                        $(
                            &Node::$type_name(Either::Right(ptr)) => {
                                let (slot, slice) = $field.1.split_first_mut().unwrap();
                                $field.1 = slice;
                                *slot = ptr
                            },
                        )*
                        _ => unreachable!(),
                    }
                }

                $( let $field = $field.0; )*

                let nodes = self.nodes;
                Ok(Yoke::attach_to_cart(Arc::new(Mutex::new(self.bump)), |_| {
                    Ast {
                        $( $field: unsafe { &*($field as *const [&$type_name]) }, )*
                        nodes: nodes.into_iter().map(
                            |(k, v)| (
                                unsafe { &*k.0 },
                                match v {
                                    $( Node::$type_name(Either::Right(ptr)) => Node::$type_name(unsafe { &*ptr }), )*
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

hydration_helper![
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
