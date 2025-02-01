use std::{mem::{size_of, MaybeUninit}, slice::from_raw_parts};
use glam::{I16Vec3, IVec3, IVec4, Mat4, U16Vec2, Vec3};
use tr_model::{tr1, tr2, tr3, tr4, tr5};

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
impl<T: ReinterpretAsBytes> ReinterpretAsBytes for MaybeUninit<T> {}

impl ReinterpretAsBytes for u8 {}
impl ReinterpretAsBytes for u16 {}
impl ReinterpretAsBytes for u32 {}
impl ReinterpretAsBytes for i32 {}
impl ReinterpretAsBytes for U16Vec2 {}
impl ReinterpretAsBytes for I16Vec3 {}
impl ReinterpretAsBytes for IVec3 {}
impl ReinterpretAsBytes for IVec4 {}
impl ReinterpretAsBytes for Vec3 {}
impl ReinterpretAsBytes for Mat4 {}
impl ReinterpretAsBytes for egui::Vec2 {}
impl ReinterpretAsBytes for tr1::Color24Bit {}
impl ReinterpretAsBytes for tr1::ObjectTexture {}
impl ReinterpretAsBytes for tr1::SpriteTexture {}
impl ReinterpretAsBytes for tr1::RoomVertex {}
impl ReinterpretAsBytes for tr1::TexturedQuad {}
impl ReinterpretAsBytes for tr1::TexturedTri {}
impl ReinterpretAsBytes for tr1::SolidQuad {}
impl ReinterpretAsBytes for tr1::SolidTri {}
impl ReinterpretAsBytes for tr2::Color32BitRgb {}
impl ReinterpretAsBytes for tr2::Color16BitArgb {}
impl ReinterpretAsBytes for tr2::RoomVertex {}
impl ReinterpretAsBytes for tr2::SolidQuad {}
impl ReinterpretAsBytes for tr2::SolidTri {}
impl ReinterpretAsBytes for tr3::RoomVertex {}
impl ReinterpretAsBytes for tr3::DsQuad {}
impl ReinterpretAsBytes for tr3::DsTri {}
impl ReinterpretAsBytes for tr4::Color32BitBgra {}
impl ReinterpretAsBytes for tr4::EffectsQuad {}
impl ReinterpretAsBytes for tr4::EffectsTri {}
impl ReinterpretAsBytes for tr4::ObjectTexture {}
impl ReinterpretAsBytes for tr5::RoomVertex {}
impl ReinterpretAsBytes for tr5::ObjectTexture {}
impl ReinterpretAsBytes for tr5::EffectsQuad {}
impl ReinterpretAsBytes for tr5::EffectsTri {}
