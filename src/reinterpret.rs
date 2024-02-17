use std::{mem::{align_of, size_of}, ptr, slice};

pub unsafe fn box_slice<A, B>(data: Box<[A]>) -> Box<[B]> {
	let len = data.len();
	assert!(data.as_ptr() as usize % align_of::<B>() == 0, "input not aligned with output type");
	assert!(len * size_of::<A>() % size_of::<B>() == 0, "{} items of size {} cannot be reinterpreted as any number of items of size {}", len, size_of::<A>(), size_of::<B>());
	Box::from_raw(ptr::slice_from_raw_parts_mut(Box::into_raw(data) as *mut B, len * size_of::<A>() / size_of::<B>()))
}

pub unsafe fn ref_to_slice<A, B>(data: &A) -> &[B] {
	assert!(data as *const A as usize % align_of::<B>() == 0, "input not aligned with output type");
	assert!(size_of::<A>() % size_of::<B>() == 0, "item of size {} cannot be reinterpreted as any number of items of size {}", size_of::<A>(), size_of::<B>());
	slice::from_raw_parts(data as *const A as *const B, size_of::<A>() / size_of::<B>())
}

pub unsafe fn slice<A, B>(data: &[A]) -> &[B] {
	let len = data.len();
	assert!(data.as_ptr() as usize % align_of::<B>() == 0, "input not aligned with output type");
	assert!(len * size_of::<A>() % size_of::<B>() == 0, "{} items of size {} cannot be reinterpreted as any number of items of size {}", len, size_of::<A>(), size_of::<B>());
	slice::from_raw_parts(data.as_ptr() as *const B, len * size_of::<A>() / size_of::<B>())
}
