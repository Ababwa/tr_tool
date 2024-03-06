extern crate self as tr_reader;

pub mod model;
pub(crate) mod impls;

use std::{io::{Cursor, Read, Result}, mem::size_of, slice};
use byteorder::{ReadBytesExt, LE};
use compress::zlib::Decoder;
pub(crate) use tr_derive::Readable;

pub(crate) trait Readable: Sized {
	fn read<R: Read>(reader: &mut R) -> Result<Self>;
}

pub(crate) fn read_boxed_slice<R: Read, T: Readable>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut vec = Vec::with_capacity(len);
	for _ in 0..len {
		vec.push(T::read(reader)?);
	}
	Ok(vec.into_boxed_slice())
}

pub(crate) unsafe fn read_boxed_slice_raw<R: Read, T>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut vec = Vec::with_capacity(len);
	vec.set_len(len);
	let buf = slice::from_raw_parts_mut(vec.as_mut_ptr() as *mut u8, len * size_of::<T>());
	reader.read_exact(buf)?;
	Ok(vec.into_boxed_slice())
}

pub(crate) trait Len {
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

pub(crate) fn read_list<R: Read, T: Readable, L: Len>(reader: &mut R) -> Result<Box<[T]>> {
	let len = L::read_len(reader)?;
	read_boxed_slice(reader, len)
}

pub(crate) fn get_zlib<R: Read>(reader: &mut R) -> Result<Decoder<Cursor<Box<[u8]>>>> {
	reader.read_u32::<LE>()?;//uncompressed_len
	let compressed_len = reader.read_u32::<LE>()? as usize;
	let bytes = read_boxed_slice::<_, u8>(reader, compressed_len)?;
	Ok(Decoder::new(Cursor::new(bytes)))
}

pub(crate) fn skip<R: Read>(reader: &mut R, num: usize) -> Result<()> {
	let mut buf = [0];
	for _ in 0..num {
		reader.read_exact(&mut buf)?;
	}
	Ok(())
}
