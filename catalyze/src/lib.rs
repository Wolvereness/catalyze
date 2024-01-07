#![doc = include_str!("../../README.md")]
#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(clippy::all, clippy::pedantic)]
#![warn(clippy::nursery)]
// #![warn(missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::result_large_err,
    clippy::enum_glob_use,
    clippy::implicit_hasher,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::missing_panics_doc, // TODO: remove after todo!()s are removed
    clippy::missing_errors_doc, // TODO: remove when I get around to documenting
	clippy::must_use_candidate, // TODO: remove once the API is settled
    clippy::wildcard_imports,
    clippy::module_inception,
	clippy::struct_excessive_bools

)]
#![cfg_attr(test, allow(clippy::too_many_lines))]

use ahash::AHasher;
use std::hash::BuildHasherDefault;

// pub mod ast;
// pub mod container;
// pub mod r#enum;
// pub mod enum_value;
// pub mod error;
// pub mod extension;
// pub mod field;
// pub mod file;
// pub mod fqn;
// pub mod generator;
// pub mod location;
// pub mod message;
// pub mod method;
// pub mod oneof;
// pub mod package;
// pub mod service;
mod ast2;

pub(crate) type HashMap<K, V> = ahash::HashMap<K, V>;
pub(crate) type HashSet<V> = ahash::HashSet<V>;
pub(crate) type IndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<AHasher>>;

macro_rules! impl_access {
    ($typ: ident, $inner: ident) => {
        impl<'ast> crate::ast::Access<'ast, $inner> for $typ<'ast> {
            fn access(&self) -> &$inner {
                self.0.access()
            }
        }
    };
}
macro_rules! impl_copy_clone {
    ($typ:ident) => {
        impl<'ast, A> Clone for $typ<'ast, A> {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }
        impl<'ast, A> Copy for $typ<'ast, A> {}
    };
}
macro_rules! impl_fqn {
    ($typ:ident) => {
        #[inherent::inherent]
        impl<'ast, A> crate::ast::Fqn for $typ<'ast, A> {
            #[doc = "Returns the [`FullyQualifiedName`] of the Message."]
            pub fn fully_qualified_name(&self) -> &crate::ast::FullyQualifiedName {
                &self.0.fqn
            }
            /// Alias for `fully_qualified_name` - returns the [`FullyQualifiedName`] of
            /// the Package.
            pub fn fqn(&self) -> &crate::ast::FullyQualifiedName {
                self.fully_qualified_name()
            }
        }
    };
}
macro_rules! impl_traits {
    ($typ: ident, $inner: ident) => {
        crate::impl_copy_clone!($typ);
        crate::impl_eq!($typ);
        crate::impl_access!($typ, $inner);
        crate::impl_fqn!($typ);
    };
}

macro_rules! impl_eq {
    ($typ:ident) => {
        impl<'ast, A> PartialEq for $typ<'ast, A> {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl<'ast, A> Eq for $typ<'ast, A> {}
    };
    () => {};
}

pub(crate) use impl_access;
pub(crate) use impl_copy_clone;
pub(crate) use impl_eq;
pub(crate) use impl_fqn;
pub(crate) use impl_traits;
