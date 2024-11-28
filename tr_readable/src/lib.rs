use std::{io::{Cursor, Read, Result}, mem::{size_of, MaybeUninit}, slice::from_raw_parts_mut};
use compress::zlib::Decoder;
use shared::alloc;

pub use tr_derive::Readable;

pub trait Readable {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()>;
}

//impl helpers

pub unsafe fn read_flat<R: Read, T>(reader: &mut R, ptr: *mut T) -> Result<()> {
	let buf = from_raw_parts_mut(ptr.cast(), size_of::<T>());
	reader.read_exact(buf)
}

pub unsafe fn read_flat_get<R: Read, T>(reader: &mut R) -> Result<T> {
	let mut val = MaybeUninit::<T>::uninit();
	read_flat(reader, val.as_mut_ptr())?;
	Ok(val.assume_init())
}

pub unsafe fn read_boxed_flat_get<R: Read, T>(reader: &mut R) -> Result<Box<T>> {
	let mut boxed = alloc::val::<T>();
	read_flat(reader, boxed.as_mut_ptr())?;
	Ok(boxed.assume_init())
}

pub unsafe fn read_boxed_flat<R: Read, T>(reader: &mut R, dest: *mut Box<T>) -> Result<()> {
	dest.write(read_boxed_flat_get(reader)?);
	Ok(())
}

pub unsafe fn read_boxed_slice_flat_get<R: Read, T>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut boxed_slice = alloc::slice::<T>(len);
	let buf = from_raw_parts_mut(boxed_slice.as_mut_ptr().cast(), size_of::<T>() * len);
	reader.read_exact(buf)?;
	Ok(boxed_slice.assume_init())
}

pub unsafe fn read_boxed_slice_flat<R: Read, T>(
	reader: &mut R, len: usize, dest: *mut Box<[T]>,
) -> Result<()> {
	dest.write(read_boxed_slice_flat_get(reader, len)?);
	Ok(())
}

pub unsafe fn read_boxed_slice_delegate<R: Read, T: Readable>(
	reader: &mut R, len: usize, dest: *mut Box<[T]>,
) -> Result<()> {
	let mut boxed_slice = alloc::slice::<T>(len);
	for i in boxed_slice.iter_mut() {
		Readable::read(reader, i.as_mut_ptr())?;
	}
	dest.write(boxed_slice.assume_init());
	Ok(())
}

pub fn zlib<R: Read>(reader: &mut R) -> Result<Decoder<Cursor<Box<[u8]>>>> {
	unsafe {
		let _uncompressed_size = read_flat_get::<_, u32>(reader)?;
		let compressed_size = read_flat_get::<_, u32>(reader)?;
		let buf = read_boxed_slice_flat_get(reader, compressed_size as usize)?;
		Ok(Decoder::new(Cursor::new(buf)))
	}
}
