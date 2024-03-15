extern crate self as tr_reader;

pub mod model;
pub(crate) mod impls;

use std::{io::{Cursor, Read, Result}, mem::size_of, slice};
use byteorder::{ReadBytesExt, LE};
use compress::zlib::Decoder;
use num_traits::AsPrimitive;
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

pub(crate) fn read_list<R: Read, T: Readable, L: Readable + AsPrimitive<usize>>(reader: &mut R) -> Result<Box<[T]>> {
	let len = L::read(reader)?.as_();
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
