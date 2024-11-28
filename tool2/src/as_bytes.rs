use std::{mem::{size_of, MaybeUninit}, ptr::slice_from_raw_parts_mut, slice::from_raw_parts};
use glam::{I16Vec3, IVec3, IVec4, Mat4, U16Vec2};
use tr_model::{tr1, tr2, tr3, tr4};

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

pub trait ToBytes {
	fn to_bytes(self) -> Box<[u8]>;
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

impl<T: ReinterpretAsBytes> ToBytes for Box<T> {
	fn to_bytes(self) -> Box<[u8]> {
		unsafe {
			Box::from_raw(slice_from_raw_parts_mut(Box::into_raw(self).cast(), size_of::<T>()))
		}
	}
}

impl<T: ReinterpretAsBytes> ToBytes for Box<[T]> {
	fn to_bytes(self) -> Box<[u8]> {
		let len = self.len() * size_of::<T>();
		unsafe {
			Box::from_raw(slice_from_raw_parts_mut(Box::into_raw(self).cast(), len))
		}
	}
}

impl<T: ReinterpretAsBytes, const N: usize> ReinterpretAsBytes for [T; N] {}
impl<T: ReinterpretAsBytes> ReinterpretAsBytes for MaybeUninit<T> {}

impl ReinterpretAsBytes for u8 {}
impl ReinterpretAsBytes for u16 {}
impl ReinterpretAsBytes for u32 {}
impl ReinterpretAsBytes for U16Vec2 {}
impl ReinterpretAsBytes for I16Vec3 {}
impl ReinterpretAsBytes for IVec3 {}
impl ReinterpretAsBytes for IVec4 {}
impl ReinterpretAsBytes for Mat4 {}
impl ReinterpretAsBytes for tr1::Color24Bit {}
impl ReinterpretAsBytes for tr1::ObjectTexture {}
impl ReinterpretAsBytes for tr1::SpriteTexture {}
impl ReinterpretAsBytes for tr1::RoomVertex {}
impl ReinterpretAsBytes for tr1::RoomQuad {}
impl ReinterpretAsBytes for tr1::RoomTri {}
impl ReinterpretAsBytes for tr1::MeshTexturedQuad {}
impl ReinterpretAsBytes for tr1::MeshTexturedTri {}
impl ReinterpretAsBytes for tr1::MeshSolidQuad {}
impl ReinterpretAsBytes for tr1::MeshSolidTri {}
impl ReinterpretAsBytes for tr2::Color32BitBGR {}
impl ReinterpretAsBytes for tr2::Color16BitARGB {}
impl ReinterpretAsBytes for tr2::RoomVertex {}
impl ReinterpretAsBytes for tr2::MeshSolidQuad {}
impl ReinterpretAsBytes for tr2::MeshSolidTri {}
impl ReinterpretAsBytes for tr3::RoomVertex {}
impl ReinterpretAsBytes for tr3::RoomQuad {}
impl ReinterpretAsBytes for tr3::RoomTri {}
impl ReinterpretAsBytes for tr4::Color32BitRGB {}
impl ReinterpretAsBytes for tr4::MeshQuad {}
impl ReinterpretAsBytes for tr4::MeshTri {}
impl ReinterpretAsBytes for tr4::ObjectTexture {}
