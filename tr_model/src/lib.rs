//! This crate depends on reinterpreting byte chunks as little endian integers.

const _: () = assert!(cfg!(target_endian = "little"));

mod u16_cursor;
pub mod tr1;
pub mod tr2;
pub mod tr3;
pub mod tr4;
pub mod tr5;

pub use tr_readable::Readable;
