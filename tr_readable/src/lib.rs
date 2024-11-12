use std::{
	alloc::{alloc, handle_alloc_error, Layout}, io::{Read, Result}, mem::{size_of, transmute, MaybeUninit},
	ops::Range, ptr::slice_from_raw_parts_mut, slice::from_raw_parts_mut,
};

pub use tr_derive::Readable;

pub trait Readable {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()>;
}

//impl helpers

unsafe fn boxed_uninit<T>() -> Box<MaybeUninit<T>> {
	let layout = Layout::new::<T>();
	let ptr = alloc(layout);
	if ptr.is_null() {
		handle_alloc_error(layout);
	}
	let ptr = ptr as *mut MaybeUninit<T>;
	Box::from_raw(ptr)
}

unsafe fn boxed_slice_uninit<T>(len: usize) -> Box<[MaybeUninit<T>]> {
	let layout = Layout::array::<T>(len).unwrap();
	let ptr = alloc(layout);
	if ptr.is_null() {
		handle_alloc_error(layout);
	}
	let ptr = slice_from_raw_parts_mut(ptr as *mut MaybeUninit<T>, len);
	Box::from_raw(ptr)
}

pub unsafe fn read_flat<R: Read, T>(reader: &mut R, ptr: *mut T) -> Result<()> {
	let buf = from_raw_parts_mut(ptr as *mut u8, size_of::<T>());
	reader.read_exact(buf)
}

pub unsafe fn read_val_flat<R: Read, T>(reader: &mut R) -> Result<T> {
	let mut val = MaybeUninit::<T>::uninit();
	read_flat(reader, val.as_mut_ptr())?;
	Ok(val.assume_init())
}

pub unsafe fn read_range_flat<R: Read, T, U>(reader: &mut R, start: *mut T, end: *mut U) -> Result<()> {
	let buf = from_raw_parts_mut(start as *mut u8, end as usize - start as usize);
	reader.read_exact(buf)
}

pub unsafe fn read_boxed_flat<R: Read, T>(reader: &mut R, dest: *mut Box<T>) -> Result<()> {
	let mut boxed = boxed_uninit::<T>();
	read_flat(reader, boxed.as_mut_ptr())?;
	let boxed = transmute::<_, Box<T>>(boxed);
	dest.write(boxed);
	Ok(())
}

pub unsafe fn read_boxed_slice_flat<R: Read, T>(
	reader: &mut R, dest: *mut Box<[T]>, len: usize,
) -> Result<()> {
	let mut boxed_slice = boxed_slice_uninit::<T>(len);
	let Range { start, end } = boxed_slice.as_mut_ptr_range();
	read_range_flat(reader, start, end)?;
	let boxed_slice = transmute::<_, Box<[T]>>(boxed_slice);
	dest.write(boxed_slice);
	Ok(())
}

pub unsafe fn read_boxed_slice_delegate<R: Read, T: Readable>(
	reader: &mut R, dest: *mut Box<[T]>, len: usize,
) -> Result<()> {
	let mut boxed_slice = boxed_slice_uninit::<T>(len);
	for i in boxed_slice.iter_mut() {
		Readable::read(reader, i.as_mut_ptr())?;
	}
	let boxed_slice = transmute::<_, Box<[T]>>(boxed_slice);
	dest.write(boxed_slice);
	Ok(())
}
