use glam::{I16Vec3, IVec3, IVec4, Mat4, U16Vec2};
use tr_model::tr1;

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

impl<T: ReinterpretAsBytes, const N: usize> ReinterpretAsBytes for [T; N] {}

impl ReinterpretAsBytes for u8 {}
impl ReinterpretAsBytes for u32 {}
impl ReinterpretAsBytes for U16Vec2 {}
impl ReinterpretAsBytes for I16Vec3 {}
impl ReinterpretAsBytes for IVec3 {}
impl ReinterpretAsBytes for IVec4 {}
impl ReinterpretAsBytes for Mat4 {}
impl ReinterpretAsBytes for tr1::RoomVertex {}
impl ReinterpretAsBytes for tr1::ObjectTexture {}
impl ReinterpretAsBytes for tr1::Color6Bit {}
impl<const N: usize> ReinterpretAsBytes for tr1::Face<N> {}
