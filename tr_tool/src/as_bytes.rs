use std::slice;

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

impl<T: ?Sized> AsBytes for T {
	fn as_bytes(&self) -> &[u8] {
		//Safety: Read-only bytes.
		unsafe {
			slice::from_raw_parts((&raw const *self).cast(), size_of_val(self))
		}
	}
}
