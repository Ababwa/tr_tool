use std::io::{Read, Result};
use arrayvec::ArrayVec;
use byteorder::{ReadBytesExt, LE};
use shared::geom::MinMax;
use glam::{
	I16Vec2, I16Vec3, I16Vec4,
	U16Vec2, U16Vec3, U16Vec4,
	IVec2, IVec3, IVec4,
	UVec2, UVec3, UVec4,
	I64Vec2, I64Vec3, I64Vec4,
	U64Vec2, U64Vec3, U64Vec4,
	Vec2, Vec3, Vec4,
	DVec2, DVec3, DVec4,
};
use nonmax::{NonMaxU8, NonMaxU16};
use crate::Readable;

//primitive impls

macro_rules! impl_readable_prim {
	($type:ty, $func:ident $(, $($endian:tt)*)?) => {
		impl Readable for $type {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				reader.$func$($($endian)*)?()
			}
		}
	};
}

macro_rules! impl_readable_prim_le {
	($type:ty, $func:ident) => {
		impl_readable_prim!($type, $func, ::<LE>);
	};
}

impl_readable_prim!(u8, read_u8);
impl_readable_prim!(i8, read_i8);
impl_readable_prim_le!(u16, read_u16);
impl_readable_prim_le!(i16, read_i16);
impl_readable_prim_le!(u32, read_u32);
impl_readable_prim_le!(i32, read_i32);
impl_readable_prim_le!(u64, read_u64);
impl_readable_prim_le!(i64, read_i64);
impl_readable_prim_le!(f32, read_f32);
impl_readable_prim_le!(f64, read_f64);

//array impls

impl<T: Readable, const N: usize> Readable for [T; N] {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let mut array = ArrayVec::new();
		for _ in 0..N {
			array.push(T::read(reader)?);
		}
		Ok(unsafe { array.into_inner().ok().unwrap_unchecked() })//reads exactly N items
	}
}

impl<T: Readable, const N: usize> Readable for Box<[T; N]> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(unsafe { crate::read_boxed_slice(reader, N)?.try_into().ok().unwrap_unchecked() })//reads exactly N items
	}
}

//nonmax impls

impl Readable for Option<NonMaxU8> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(NonMaxU8::new(reader.read_u8()?))
	}
}

impl Readable for Option<NonMaxU16> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(NonMaxU16::new(reader.read_u16::<LE>()?))
	}
}

//minmax impl

impl<T: Readable> Readable for MinMax<T> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(MinMax { min: T::read(reader)?, max: T::read(reader)? })
	}
}

//glam impls

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
