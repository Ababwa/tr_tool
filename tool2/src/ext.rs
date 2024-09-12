use glam::{IVec2, Mat4};
use tr_model::tr1;

use crate::PaddedQuad;

pub trait IntoValIter<T> {
	fn into_val_iter(self) -> <Vec<T> as IntoIterator>::IntoIter;
}

impl<T> IntoValIter<T> for Box<[T]> {
	fn into_val_iter(self) -> <Vec<T> as IntoIterator>::IntoIter {
		self.into_vec().into_iter()
	}
}

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

pub trait ReinterpretAsBytes {}

impl<T: ReinterpretAsBytes> AsBytes for T {
	fn as_bytes(&self) -> &[u8] {
		unsafe { reinterpret::ref_to_slice(self) }
	}
}

impl<T: ReinterpretAsBytes> AsBytes for [T] {
	fn as_bytes(&self) -> &[u8] {
		unsafe { reinterpret::slice(self) }
	}
}

impl ReinterpretAsBytes for u32 {}
impl ReinterpretAsBytes for Mat4 {}
impl ReinterpretAsBytes for IVec2 {}
impl ReinterpretAsBytes for PaddedQuad {}
impl ReinterpretAsBytes for tr1::RoomVertex {}
impl ReinterpretAsBytes for tr1::ObjectTexture {}
