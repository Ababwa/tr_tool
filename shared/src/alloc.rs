use std::{alloc::{alloc, handle_alloc_error, Layout}, mem::MaybeUninit, ptr::slice_from_raw_parts_mut};

pub fn val<T>() -> Box<MaybeUninit<T>> {
	unsafe {
		let layout = Layout::new::<T>();
		let ptr = alloc(layout);
		if ptr.is_null() {
			handle_alloc_error(layout);
		}
		Box::from_raw(ptr.cast())
	}
}

pub fn array<T, const N: usize>() -> Box<[MaybeUninit<T>; N]> {
	unsafe { val().assume_init() }
}

pub fn slice<T>(len: usize) -> Box<[MaybeUninit<T>]> {
	unsafe {
		let layout = Layout::array::<T>(len).unwrap();
		let ptr = alloc(layout);
		if ptr.is_null() {
			handle_alloc_error(layout);
		}
		Box::from_raw(slice_from_raw_parts_mut(ptr as *mut MaybeUninit<T>, len))
	}
}
