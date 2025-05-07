use std::{
	io::{Cursor, Read, Result, Seek, SeekFrom}, mem::{size_of, MaybeUninit}, slice::from_raw_parts_mut,
};
use compress::zlib::Decoder;

pub use tr_derive::Readable;

pub trait Readable {
	unsafe fn read<R: Read + Seek>(reader: &mut R, this: *mut Self) -> Result<()>;
}

pub trait ToLen {
	fn get_len(&self) -> usize;
}

impl<T> ToLen for Box<[T]> {
	fn get_len(&self) -> usize {
		self.len()
	}
}

macro_rules! impl_to_len_prim {
	($type:ty) => {
		impl ToLen for $type {
			fn get_len(&self) -> usize {
				*self as usize
			}
		}
	};
}

impl_to_len_prim!(u16);
impl_to_len_prim!(u32);

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

pub unsafe fn read_slice_get<R: Read, T>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut slice = Box::new_uninit_slice(len);
	read_into_slice(reader, slice.as_mut_ptr(), len)?;
	Ok(slice.assume_init())
}

pub fn zlib<R: Read + Seek>(reader: &mut R) -> Result<Cursor<Box<[u8]>>> {
	unsafe {
		let uncompressed_size = read_get::<_, u32>(reader)?;
		let compressed_size = read_get::<_, u32>(reader)?;
		let start = reader.stream_position()?;
		let mut slice = Box::new_uninit_slice(uncompressed_size as usize);
		let mut zlib_reader = Decoder::new(reader.take(compressed_size as u64));
		read_into_slice(&mut zlib_reader, slice.as_mut_ptr(), slice.len())?;
		reader.seek(SeekFrom::Start(start + compressed_size as u64))?;
		Ok(Cursor::new(slice.assume_init()))
	}
}
