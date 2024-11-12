use std::{mem::size_of, slice::from_raw_parts};
use glam::{I16Vec3, IVec3, IVec4, Mat4, U16Vec2};
use tr_model::{tr1, tr2, tr3};

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

pub trait ReinterpretAsBytes {}

impl<T: ReinterpretAsBytes> AsBytes for T {
	fn as_bytes(&self) -> &[u8] {
		unsafe {
			from_raw_parts((self as *const T).cast(), size_of::<T>())
		}
	}
}

impl<T: ReinterpretAsBytes> AsBytes for [T] {
	fn as_bytes(&self) -> &[u8] {
		unsafe {
			from_raw_parts(self.as_ptr().cast(), self.len() * size_of::<T>())
		}
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
impl ReinterpretAsBytes for tr1::SpriteTexture {}
impl ReinterpretAsBytes for tr1::Color24Bit {}
impl ReinterpretAsBytes for tr1::RoomQuad {}
impl ReinterpretAsBytes for tr1::RoomTri {}
impl ReinterpretAsBytes for tr1::MeshTexturedQuad {}
impl ReinterpretAsBytes for tr1::MeshTexturedTri {}
impl ReinterpretAsBytes for tr1::MeshSolidQuad {}
impl ReinterpretAsBytes for tr1::MeshSolidTri {}
impl ReinterpretAsBytes for tr2::RoomVertex {}
impl ReinterpretAsBytes for tr2::MeshSolidQuad {}
impl ReinterpretAsBytes for tr2::MeshSolidTri {}
impl ReinterpretAsBytes for tr2::Color16Bit {}
impl ReinterpretAsBytes for tr3::RoomVertex {}
