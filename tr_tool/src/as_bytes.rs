use std::slice;

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

impl<T: ?Sized> AsBytes for T {
	fn as_bytes(&self) -> &[u8] {
		unsafe {
			slice::from_raw_parts((self as *const T).cast(), size_of_val(self))
		}
	}
}
