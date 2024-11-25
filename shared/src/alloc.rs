use std::{alloc::{alloc, handle_alloc_error, Layout}, mem::MaybeUninit, ptr::slice_from_raw_parts_mut};

pub fn val<T>() -> Box<MaybeUninit<T>> {
	unsafe {
		let layout = Layout::new::<T>();
		let ptr = alloc(layout);
		if ptr.is_null() {
			handle_alloc_error(layout);
		}
		let ptr = ptr as *mut MaybeUninit<T>;
		Box::from_raw(ptr)
	}
}

pub fn slice<T>(len: usize) -> Box<[MaybeUninit<T>]> {
	unsafe {
		let layout = Layout::array::<T>(len).unwrap();
		let ptr = alloc(layout);
		if ptr.is_null() {
			handle_alloc_error(layout);
		}
		let ptr = slice_from_raw_parts_mut(ptr as *mut MaybeUninit<T>, len);
		Box::from_raw(ptr)
	}
}
