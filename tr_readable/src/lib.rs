use std::{io::{Cursor, Read, Result}, mem::size_of, slice};
use arrayvec::ArrayVec;
use byteorder::{ReadBytesExt, LE};
use compress::zlib::Decoder;
use num_traits::AsPrimitive;
use glam::{I16Vec2, I16Vec3, IVec3, U16Vec2, Vec3};
use glam_traits::ext::U8Vec2;
use nonmax::{NonMaxU8, NonMaxU16};
use shared::MinMax;

pub use tr_derive::Readable;

pub trait Readable: Sized {
	fn read<R: Read>(reader: &mut R) -> Result<Self>;
}

//impl helpers

pub fn read_boxed_slice<R: Read, T: Readable>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut vec = Vec::with_capacity(len);
	for _ in 0..len {
		vec.push(T::read(reader)?);
	}
	Ok(vec.into_boxed_slice())
}

pub fn read_list<R: Read, T: Readable, L: Readable + AsPrimitive<usize>>(reader: &mut R) -> Result<Box<[T]>> {
	let len = L::read(reader)?.as_();
	read_boxed_slice(reader, len)
}

pub fn get_zlib<R: Read>(reader: &mut R) -> Result<Decoder<Cursor<Box<[u8]>>>> {
	reader.read_u32::<LE>()?;//uncompressed_len
	let compressed_len = reader.read_u32::<LE>()? as usize;
	let bytes = read_boxed_slice(reader, compressed_len)?;
	Ok(Decoder::new(Cursor::new(bytes)))
}

pub fn skip<R: Read>(reader: &mut R, num: usize) -> Result<()> {
	let mut buf = [0];
	for _ in 0..num {
		reader.read_exact(&mut buf)?;
	}
	Ok(())
}

//flat

pub fn read_boxed_slice_flat<R: Read, T>(reader: &mut R, len: usize) -> Result<Box<[T]>> {
	let mut vec = Vec::with_capacity(len);
	unsafe {
		vec.set_len(len);
		let buf = slice::from_raw_parts_mut(vec.as_mut_ptr() as *mut u8, len * size_of::<T>());
		reader.read_exact(buf)?;
	}
	Ok(vec.into_boxed_slice())
}

pub fn read_list_flat<R: Read, T, L: Readable + AsPrimitive<usize>>(reader: &mut R) -> Result<Box<[T]>> {
	let len = L::read(reader)?.as_();
	read_boxed_slice_flat(reader, len)
}

pub fn read_boxed_array_flat<R: Read, T, const N: usize>(reader: &mut R) -> Result<Box<[T; N]>> {
	Ok(read_boxed_slice_flat(reader, N)?.try_into().ok().unwrap())//reads exactly N items
}

//primitive impls

macro_rules! impl_readable_prim {
	($type:ty, $func:ident $(, $endian:ty)?) => {
		impl Readable for $type {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				reader.$func$(::<$endian>)?()
			}
		}
	};
}

impl_readable_prim!(u8, read_u8);
impl_readable_prim!(i8, read_i8);
impl_readable_prim!(u16, read_u16, LE);
impl_readable_prim!(i16, read_i16, LE);
impl_readable_prim!(u32, read_u32, LE);
impl_readable_prim!(i32, read_i32, LE);
impl_readable_prim!(u64, read_u64, LE);
impl_readable_prim!(i64, read_i64, LE);
impl_readable_prim!(f32, read_f32, LE);
impl_readable_prim!(f64, read_f64, LE);

//array impls

impl<T: Readable, const N: usize> Readable for [T; N] {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let mut array = ArrayVec::new();
		for _ in 0..N {
			array.push(T::read(reader)?);
		}
		Ok(array.into_inner().ok().unwrap())//reads exactly N items
	}
}

impl<T: Readable, const N: usize> Readable for Box<[T; N]> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(read_boxed_slice(reader, N)?.try_into().ok().unwrap())//reads exactly N items
	}
}

//nonmax impls

macro_rules! impl_nonmax {
	($type:ty, $func:ident $(, $endian:ty)?) => {
		impl Readable for Option<$type> {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				Ok(<$type>::new(reader.$func$(::<$endian>)?()?))
			}
		}
	};
}

impl_nonmax!(NonMaxU8, read_u8);
impl_nonmax!(NonMaxU16, read_u16, LE);

//minmax impl

impl<T: Readable> Readable for MinMax<T> {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(Self { min: T::read(reader)?, max: T::read(reader)? })
	}
}

//glam impls

macro_rules! impl_readable_glam {
	($type:ty, $prim:ty, $n:literal) => {
		impl Readable for $type {
			fn read<R: Read>(reader: &mut R) -> Result<Self> {
				Ok(<[$prim; $n]>::read(reader)?.into())
			}
		}
	};
}

impl_readable_glam!(U8Vec2, u8, 2);
impl_readable_glam!(U16Vec2, u16, 2);
impl_readable_glam!(I16Vec2, i16, 2);
impl_readable_glam!(I16Vec3, i16, 3);
impl_readable_glam!(IVec3, i32, 3);
impl_readable_glam!(Vec3, f32, 3);
