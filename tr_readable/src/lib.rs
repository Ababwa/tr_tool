use std::{io::{Cursor, Read, Result}, mem::{size_of, MaybeUninit}, slice::from_raw_parts_mut};
use compress::zlib::Decoder;
use shared::alloc;

pub use tr_derive::Readable;

pub trait Readable {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()>;
}

pub trait ToLen {
	fn get_len(&self) -> usize;
}

impl<T> ToLen for Box<[T]> {
	fn get_len(&self) -> usize {
		self.len()
	}
}

//impl helpers

pub unsafe fn read_into<R: Read, T>(reader: &mut R, ptr: *mut T) -> Result<()> {
	let buf = from_raw_parts_mut(ptr.cast(), size_of::<T>());
	reader.read_exact(buf)
}

pub unsafe fn read_into_slice<R: Read, T>(reader: &mut R, ptr: *mut T, len: usize) -> Result<()> {
	let buf = from_raw_parts_mut(ptr.cast(), size_of::<T>() * len);
	reader.read_exact(buf)
}

pub unsafe fn read_get<R: Read, T>(reader: &mut R) -> Result<T> {
	let mut val = MaybeUninit::<T>::uninit();
	read_into(reader, val.as_mut_ptr())?;
	Ok(val.assume_init())
}

pub fn zlib<R: Read>(reader: &mut R) -> Result<Decoder<Cursor<Box<[u8]>>>> {
	unsafe {
		let _uncompressed_size = read_get::<_, u32>(reader)?;
		let compressed_size = read_get::<_, u32>(reader)?;
		let mut slice = alloc::slice(compressed_size as usize);
		read_into_slice(reader, slice.as_mut_ptr(), slice.len())?;
		Ok(Decoder::new(Cursor::new(slice.assume_init())))
	}
}
