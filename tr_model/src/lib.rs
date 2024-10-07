#[cfg(target_endian = "big")]
const _: () = panic!("big endian not supported");

pub mod tr1;
mod u16_cursor;

pub use tr_readable::Readable;
