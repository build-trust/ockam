#![deny(unsafe_code)]
#![allow(missing_docs, dead_code)]
#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces, unused_qualifications)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod tcp_interceptor;
