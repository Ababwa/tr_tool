use std::ptr;

pub mod packing;
pub mod geom;
pub mod ortho;
pub mod vtx_attr;
pub mod vec_convert;

pub const IMG_DIM_U32: u32 = tr_reader::tr4::IMG_DIM as u32;

pub fn flatten<const N: usize>(data: Box<[[u8; N]]>) -> Box<[u8]> {
	let len = data.len();
	let data = Box::into_raw(data) as *mut u8;
	unsafe {
		let data = ptr::slice_from_raw_parts_mut(data, len * N);
		Box::from_raw(data)
	}
}
