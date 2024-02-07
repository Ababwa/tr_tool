use std::io::{Read, Result};
use glam_traits::glam::{
	I16Vec2, I16Vec3, I16Vec4,
	U16Vec2, U16Vec3, U16Vec4,
	IVec2, IVec3, IVec4,
	UVec2, UVec3, UVec4,
	I64Vec2, I64Vec3, I64Vec4,
	U64Vec2, U64Vec3, U64Vec4,
	Vec2, Vec3, Vec4,
	DVec2, DVec3, DVec4,
};
use crate::Readable;

macro_rules! impl_readable_glam {
	($type:ty, $array:ty) => {
		impl Readable for $type {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				Ok(<$array>::read(reader)?.into())
			}
		}
	};
}

impl_readable_glam!(I16Vec2, [i16; 2]);
impl_readable_glam!(I16Vec3, [i16; 3]);
impl_readable_glam!(I16Vec4, [i16; 4]);

impl_readable_glam!(U16Vec2, [u16; 2]);
impl_readable_glam!(U16Vec3, [u16; 3]);
impl_readable_glam!(U16Vec4, [u16; 4]);

impl_readable_glam!(IVec2, [i32; 2]);
impl_readable_glam!(IVec3, [i32; 3]);
impl_readable_glam!(IVec4, [i32; 4]);

impl_readable_glam!(UVec2, [u32; 2]);
impl_readable_glam!(UVec3, [u32; 3]);
impl_readable_glam!(UVec4, [u32; 4]);

impl_readable_glam!(I64Vec2, [i64; 2]);
impl_readable_glam!(I64Vec3, [i64; 3]);
impl_readable_glam!(I64Vec4, [i64; 4]);

impl_readable_glam!(U64Vec2, [u64; 2]);
impl_readable_glam!(U64Vec3, [u64; 3]);
impl_readable_glam!(U64Vec4, [u64; 4]);

impl_readable_glam!(Vec2, [f32; 2]);
impl_readable_glam!(Vec3, [f32; 3]);
impl_readable_glam!(Vec4, [f32; 4]);

impl_readable_glam!(DVec2, [f64; 2]);
impl_readable_glam!(DVec3, [f64; 3]);
impl_readable_glam!(DVec4, [f64; 4]);
