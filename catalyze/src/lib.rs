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

use std::{hash::BuildHasherDefault, ops::Deref, rc::Weak};

use ahash::AHasher;
use node::{Downgrade, Upgrade};

pub mod ast;
pub mod container;
pub mod r#enum;
pub mod enum_value;
pub mod error;
pub mod extension;
pub mod field;
pub mod file;
pub mod fqn;
pub mod generator;
pub mod location;
pub mod message;
pub mod method;
pub mod node;
pub mod oneof;
pub mod package;
pub mod service;
pub mod uninterpreted_option;

pub(crate) type HashMap<K, V> = ahash::HashMap<K, V>;
pub(crate) type HashSet<V> = ahash::HashSet<V>;

pub(crate) type IndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<AHasher>>;
