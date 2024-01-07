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
    ($([$f1:ident, $f2:ident]($d:tt => $t:tt),)*) => {$(
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
    )*};
}

impl AstHydration {
    nodefn![
        [add_package, placeholder_package](PackageData => Package),
        [add_file, placeholder_file](FileData => File),
        [add_message, placeholder_message](MessageData => Message),
        [add_enum, placeholder_enum](EnumData => Enum),
        [add_enum_value, placeholder_enum_value](EnumValueData => EnumValue),
        [add_service, placeholder_service](ServiceData => Service),
        [add_method, placeholder_method](MethodData => Method),
        [add_field, placeholder_field](FieldData => Field),
        [add_oneof, placeholder_oneof](OneofData => Oneof),
        [add_extension, placeholder_extension](ExtensionData => Extension),
    ];

    pub fn finish(self) -> Result<Yoke<Ast<'static>, ErasedArcCart>, Error> {
        let mut count_package = 0;
        let mut count_file = 0;
        let mut count_message = 0;
        let mut count_enum = 0;
        let mut count_enum_value = 0;
        let mut count_service = 0;
        let mut count_method = 0;
        let mut count_field = 0;
        let mut count_oneof = 0;
        let mut count_extension = 0;
        for node in self.nodes.values() {
            match node {
                Node::Package(Either::Left(_)) |
                Node::File(Either::Left(_)) |
                Node::Message(Either::Left(_)) |
                Node::Enum(Either::Left(_)) |
                Node::EnumValue(Either::Left(_)) |
                Node::Service(Either::Left(_)) |
                Node::Method(Either::Left(_)) |
                Node::Field(Either::Left(_)) |
                Node::Oneof(Either::Left(_)) |
                Node::Extension(Either::Left(_)) =>
                    return Err(Error::Incomplete),
                Node::Package(Either::Right(_)) =>
                    count_package += 1,
                Node::File(Either::Right(_)) =>
                    count_file += 1,
                Node::Message(Either::Right(_)) =>
                    count_message += 1,
                Node::Enum(Either::Right(_)) =>
                    count_enum += 1,
                Node::EnumValue(Either::Right(_)) =>
                    count_enum_value += 1,
                Node::Service(Either::Right(_)) =>
                    count_service += 1,
                Node::Method(Either::Right(_)) =>
                    count_method += 1,
                Node::Field(Either::Right(_)) =>
                    count_field += 1,
                Node::Oneof(Either::Right(_)) =>
                    count_oneof += 1,
                Node::Extension(Either::Right(_)) =>
                    count_extension += 1,
            }
        }

        fn null_ptr<I, R>(_: I) -> *const R { ptr::null() }
        let mut slice_package = self.bump.alloc_slice_fill_with(count_package, null_ptr);
        let ptr_package: *const [*const Package] = slice_package;
        let mut slice_file = self.bump.alloc_slice_fill_with(count_file, null_ptr);
        let ptr_file: *const [*const File] = slice_file;
        let mut slice_message = self.bump.alloc_slice_fill_with(count_message, null_ptr);
        let ptr_message: *const [*const Message] = slice_message;
        let mut slice_enum = self.bump.alloc_slice_fill_with(count_enum, null_ptr);
        let ptr_enum: *const [*const Enum] = slice_enum;
        let mut slice_enum_value = self.bump.alloc_slice_fill_with(count_enum_value, null_ptr);
        let ptr_enum_value: *const [*const EnumValue] = slice_enum_value;
        let mut slice_service = self.bump.alloc_slice_fill_with(count_service, null_ptr);
        let ptr_service: *const [*const Service] = slice_service;
        let mut slice_method = self.bump.alloc_slice_fill_with(count_method, null_ptr);
        let ptr_method: *const [*const Method] = slice_method;
        let mut slice_field = self.bump.alloc_slice_fill_with(count_field, null_ptr);
        let ptr_field: *const [*const Field] = slice_field;
        let mut slice_oneof = self.bump.alloc_slice_fill_with(count_oneof, null_ptr);
        let ptr_oneof: *const [*const Oneof] = slice_oneof;
        let mut slice_extension = self.bump.alloc_slice_fill_with(count_extension, null_ptr);
        let ptr_extension: *const [*const Extension] = slice_extension;

        for node in self.nodes.values() {
            match node {
                &Node::Package(Either::Right(ptr)) => {
                    let (slot, slice) = slice_package.split_first_mut().unwrap();
                    slice_package = slice;
                    *slot = ptr
                },
                &Node::File(Either::Right(ptr)) => {
                    let (slot, slice) = slice_file.split_first_mut().unwrap();
                    slice_file = slice;
                    *slot = ptr
                },
                &Node::Message(Either::Right(ptr)) => {
                    let (slot, slice) = slice_message.split_first_mut().unwrap();
                    slice_message = slice;
                    *slot = ptr
                },
                &Node::Enum(Either::Right(ptr)) => {
                    let (slot, slice) = slice_enum.split_first_mut().unwrap();
                    slice_enum = slice;
                    *slot = ptr
                },
                &Node::EnumValue(Either::Right(ptr)) => {
                    let (slot, slice) = slice_enum_value.split_first_mut().unwrap();
                    slice_enum_value = slice;
                    *slot = ptr
                },
                &Node::Service(Either::Right(ptr)) => {
                    let (slot, slice) = slice_service.split_first_mut().unwrap();
                    slice_service = slice;
                    *slot = ptr
                },
                &Node::Method(Either::Right(ptr)) => {
                    let (slot, slice) = slice_method.split_first_mut().unwrap();
                    slice_method = slice;
                    *slot = ptr
                },
                &Node::Field(Either::Right(ptr)) => {
                    let (slot, slice) = slice_field.split_first_mut().unwrap();
                    slice_field = slice;
                    *slot = ptr
                },
                &Node::Oneof(Either::Right(ptr)) => {
                    let (slot, slice) = slice_oneof.split_first_mut().unwrap();
                    slice_oneof = slice;
                    *slot = ptr
                },
                &Node::Extension(Either::Right(ptr)) => {
                    let (slot, slice) = slice_extension.split_first_mut().unwrap();
                    slice_extension = slice;
                    *slot = ptr
                },
                _ => unreachable!(),
            }
        }

        let nodes = self.nodes;
        Ok(Yoke::attach_to_cart(Arc::new(Mutex::new(self.bump)), |_| {
            Ast {
                packages: unsafe { &*(ptr_package as *const [&Package]) },
                files: unsafe { &*(ptr_file as *const [&File]) },
                messages: unsafe { &*(ptr_message as *const [&Message]) },
                enums: unsafe { &*(ptr_enum as *const [&Enum]) },
                enum_values: unsafe { &*(ptr_enum_value as *const [&EnumValue]) },
                services: unsafe { &*(ptr_service as *const [&Service]) },
                methods: unsafe { &*(ptr_method as *const [&Method]) },
                fields: unsafe { &*(ptr_field as *const [&Field]) },
                oneofs: unsafe { &*(ptr_oneof as *const [&Oneof]) },
                extensions: unsafe { &*(ptr_extension as *const [&Extension]) },
                nodes: nodes.into_iter().map(
                    |(k, v)| (
                        unsafe { &*k.0 },
                        match v {
                            Node::Package(Either::Right(ptr)) =>
                                Node::Package(unsafe { &*ptr }),
                            Node::File(Either::Right(ptr)) =>
                                Node::File(unsafe { &*ptr }),
                            Node::Message(Either::Right(ptr)) =>
                                Node::Message(unsafe { &*ptr }),
                            Node::Enum(Either::Right(ptr)) =>
                                Node::Enum(unsafe { &*ptr }),
                            Node::EnumValue(Either::Right(ptr)) =>
                                Node::EnumValue(unsafe { &*ptr }),
                            Node::Service(Either::Right(ptr)) =>
                                Node::Service(unsafe { &*ptr }),
                            Node::Method(Either::Right(ptr)) =>
                                Node::Method(unsafe { &*ptr }),
                            Node::Field(Either::Right(ptr)) =>
                                Node::Field(unsafe { &*ptr }),
                            Node::Oneof(Either::Right(ptr)) =>
                                Node::Oneof(unsafe { &*ptr }),
                            Node::Extension(Either::Right(ptr)) =>
                                Node::Extension(unsafe { &*ptr }),
                            _ => unreachable!(),
                        },
                    )
                ).collect(),
            }
        }))
    }
}
