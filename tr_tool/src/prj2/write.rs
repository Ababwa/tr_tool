use std::{fmt::Debug, io::{Result, Write}, path::PathBuf};
use crate::as_bytes::AsBytes;

/// `T` is a discriminant to enable multiple blanket impls.
pub trait ToBytes<T> {
	type Bytes: AsRef<[u8]>;
	fn to_bytes(self) -> Result<Self::Bytes>;
}

// impl ToBytes<[(); 0]> for &[u8] {
// 	type Bytes = Self;
// 	fn to_bytes(self) -> Result<Self::Bytes> {
// 		Ok(self)
// 	}
// }

impl ToBytes<[(); 0]> for &str {
	type Bytes = Self;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(self)
	}
}

impl ToBytes<[(); 0]> for PathBuf {
	type Bytes = String;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(self.into_os_string().into_string().expect("not UTF-8"))
	}
}

pub struct Leb128Bytes([u8; 9], u8);

impl AsRef<[u8]> for Leb128Bytes {
	fn as_ref(&self) -> &[u8] {
		&self.0[..self.1 as usize]
	}
}

pub struct Leb128<T>(pub T);

impl<T> ToBytes<[(); 0]> for Leb128<T> where T: TryInto<i64>, T::Error: Debug {
	type Bytes = Leb128Bytes;
	fn to_bytes(self) -> Result<Self::Bytes> {
		let mut bytes = [0; 9];
		let len = bytes.as_mut_slice().leb128(self.0)?;
		Ok(Leb128Bytes(bytes, len as u8))
	}
}

impl<F: FnOnce(&mut Vec<u8>) -> Result<()>> ToBytes<[(); 0]> for F {
	type Bytes = Vec<u8>;
	fn to_bytes(self) -> Result<Self::Bytes> {
		let mut vec = vec![];
		self(&mut vec)?;
		Ok(vec)
	}
}

pub struct AsBytesRef<T>(T);

impl<T: AsBytes> AsRef<[u8]> for AsBytesRef<T> {
	fn as_ref(&self) -> &[u8] {
		self.0.as_bytes()
	}
}

impl<T: AsBytes> ToBytes<[(); 1]> for T {
	type Bytes = AsBytesRef<T>;
	fn to_bytes(self) -> Result<Self::Bytes> {
		Ok(AsBytesRef(self))
	}
}

pub struct ChunkWriter<'a, T>(&'a mut T);

impl<'a, T: Write> ChunkWriter<'a, T> {
	pub fn chunk<A, D: ToBytes<A>>(&mut self, id: &[u8], data: D) -> Result<()> {
		self.0.leb128(id.len())?;
		self.0.write_all(id)?;
		let data = data.to_bytes()?;
		let data = data.as_ref();
		self.0.leb128(data.len())?;
		self.0.write_all(data)?;
		Ok(())
	}
}

pub trait WriteExt: Write + Sized {
	fn leb128<N>(&mut self, num: N) -> Result<usize> where N: TryInto<i64>, N::Error: Debug {
		leb128::write::signed(self, num.try_into().unwrap())
	}
	
	fn chunk_stream<F: FnOnce(ChunkWriter<Self>) -> Result<()>>(&mut self, f: F) -> Result<()> {
		f(ChunkWriter(self))?;
		self.write_all(&[0])?;
		Ok(())
	}
}

impl<W: Write> WriteExt for W {}
