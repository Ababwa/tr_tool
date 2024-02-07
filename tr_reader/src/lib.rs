extern crate self as tr_reader;

pub mod glam_impls;
pub mod tr4;

use std::io::{Read, Cursor, Result};
use arrayvec::ArrayVec;
use byteorder::{ReadBytesExt, LE};
use compress::zlib::Decoder;

pub use tr_derive::Readable;

pub trait Readable {
	fn read<R: Read>(reader: &mut R) -> Result<Self> where Self: Sized;
}

macro_rules! impl_readable_prim {
	($type:ty, $func:ident $(, $($endian:tt)*)?) => {
		impl Readable for $type {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				reader.$func$($($endian)*)?()
			}
		}
	};
}

macro_rules! impl_readable_prim_le {
	($type:ty, $func:ident) => {
		impl_readable_prim!($type, $func, ::<LE>);
	};
}

impl_readable_prim!(u8, read_u8);
impl_readable_prim!(i8, read_i8);
impl_readable_prim_le!(u16, read_u16);
impl_readable_prim_le!(i16, read_i16);
impl_readable_prim_le!(u32, read_u32);
impl_readable_prim_le!(i32, read_i32);
impl_readable_prim_le!(u64, read_u64);
impl_readable_prim_le!(i64, read_i64);
impl_readable_prim_le!(f32, read_f32);
impl_readable_prim_le!(f64, read_f64);

impl<T: Readable, const N: usize> Readable for [T; N] {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let mut array = ArrayVec::new();
		for _ in 0..N {
			array.push(T::read(reader)?);
		}
		Ok(unsafe { array.into_inner().ok().unwrap_unchecked() })//reads exactly N items
	}
}

pub fn read_boxed_slice<R: Read, T: Readable>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut vec = Vec::with_capacity(len);
	for _ in 0..len {
		vec.push(T::read(reader)?);
	}
	Ok(vec.into_boxed_slice())
}

impl<T: Readable, const N: usize> Readable for Box<[T; N]> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(unsafe { read_boxed_slice(reader, N)?.try_into().ok().unwrap_unchecked() })//reads exactly N items
	}
}

pub trait Len {
	fn read_len<R: Read>(reader: &mut R) -> Result<usize>;
}

macro_rules! impl_len {
	($type:ty, $func:ident) => {
		impl Len for $type {
			fn read_len<R: Read>(reader: &mut R) -> Result<usize> {
				Ok(reader.$func::<LE>()? as usize)
			}
		}
	};
}

impl_len!(u16, read_u16);
impl_len!(u32, read_u32);

pub fn read_list<R: Read, T: Readable, L: Len>(reader: &mut R) -> Result<Box<[T]>> {
	let len = L::read_len(reader)?;
	read_boxed_slice(reader, len)
}

pub fn get_zlib<R: Read>(reader: &mut R) -> Result<Decoder<Cursor<Box<[u8]>>>> {
	reader.read_u32::<LE>()?;//uncompressed_len
	let compressed_len = reader.read_u32::<LE>()? as usize;
	let bytes = read_boxed_slice::<_, u8>(reader, compressed_len)?;
	Ok(Decoder::new(Cursor::new(bytes)))
}

pub fn skip<R: Read, const N: usize>(reader: &mut R) -> Result<()> {
	let mut buf = [0];
	for _ in 0..N {
		reader.read_exact(&mut buf)?;
	}
	Ok(())
}
