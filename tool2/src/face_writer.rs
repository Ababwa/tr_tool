use std::ops::Range;
use tr_model::tr1;
use crate::{data_writer::FaceArrayRef, multi_cursor::MultiCursorBuffer, FaceOffsets, MeshFaceArrayRefs};

//512 KB
const FACES_SIZE: usize = 524288;

//0..262144, len: 262144 (1/2 buffer)
const TEXTURED_QUADS_CURSOR: usize = 0;
const TEXTURED_QUADS_OFFSET: usize = 0;

//262144..393216, len: 131072 (1/4 buffer)
const TEXTURED_TRIS_CURSOR: usize = 1;
const TEXTURED_TRIS_OFFSET: usize = 262144;

//393216..458752, len: 65536 (1/8 buffer)
const SOLID_QUADS_CURSOR: usize = 2;
const SOLID_QUADS_OFFSET: usize = 393216;

//458752..524288, len: 65536 (1/8 buffer)
const SOLID_TRIS_CURSOR: usize = 3;
const SOLID_TRIS_OFFSET: usize = 458752;

const FACE_INSTANCE_SIZE: u32 = 8;

/*
face instance:
0000000000000000TTTTTTTTTTTTTTTTFFFFFFFFFFFFFFFFAAAAAAAAAAAAAAAA
A: face array index (16 bits)
F: face index (16 bits)
T: transform index (16 bits)
*/
fn face_instance(face_array_index: u32, face_index: u32, transform_index: u32) -> u64 {
	assert!(face_array_index < 65536, "face_array_index: {}", face_array_index);
	assert!(face_index < 65536, "face_index: {}", face_index);
	assert!(transform_index < 65536, "transform_index: {}", transform_index);
	face_array_index as u64 | ((face_index as u64) << 16) | ((transform_index as u64) << 32)
}

fn to_instance_range(byte_range: Range<usize>) -> Range<u32> {
	byte_range.start as u32 / FACE_INSTANCE_SIZE..byte_range.end as u32 / FACE_INSTANCE_SIZE
}

mod private {
	pub trait Face { const CURSOR: usize; }
}

use private::Face;

impl Face for tr1::RoomQuad { const CURSOR: usize = TEXTURED_QUADS_CURSOR; }
impl Face for tr1::RoomTri { const CURSOR: usize = TEXTURED_TRIS_CURSOR; }
impl Face for tr1::MeshTexturedQuad { const CURSOR: usize = TEXTURED_QUADS_CURSOR; }
impl Face for tr1::MeshTexturedTri { const CURSOR: usize = TEXTURED_TRIS_CURSOR; }
impl Face for tr1::MeshSolidQuad { const CURSOR: usize = SOLID_QUADS_CURSOR; }
impl Face for tr1::MeshSolidTri { const CURSOR: usize = SOLID_TRIS_CURSOR; }

pub struct FaceWriter {
	mc: MultiCursorBuffer,
}

impl FaceWriter {
	pub fn new() -> Self {
		Self {
			mc: MultiCursorBuffer::new(
				FACES_SIZE,
				&[TEXTURED_QUADS_OFFSET, TEXTURED_TRIS_OFFSET, SOLID_QUADS_OFFSET, SOLID_TRIS_OFFSET],
			),
		}
	}
	
	pub fn write_face_instance_array<F>(&mut self, face_array_ref: FaceArrayRef<F>, transform_index: u32)
	where F: Face {
		let mut face_type_writer = self.mc.get_writer(F::CURSOR);
		for face_index in 0..face_array_ref.len {
			if let Err(e) = face_type_writer.write(
				&face_instance(face_array_ref.index, face_index, transform_index).to_le_bytes(),
			) {
				panic!("write face instance fail: type: {}, msg: {}", F::CURSOR, e);
			}
		}
	}
	
	pub fn write_mesh(&mut self, mesh: &MeshFaceArrayRefs, transform_index: u32) {
		self.write_face_instance_array(mesh.textured_quads, transform_index);
		self.write_face_instance_array(mesh.textured_tris, transform_index);
		self.write_face_instance_array(mesh.solid_quads, transform_index);
		self.write_face_instance_array(mesh.solid_tris, transform_index);
	}
	
	pub fn get_offsets(&self) -> FaceOffsets {
		FaceOffsets {
			textured_quads: self.mc.get_pos(TEXTURED_QUADS_CURSOR) as u32 / FACE_INSTANCE_SIZE,
			textured_tris: self.mc.get_pos(TEXTURED_TRIS_CURSOR) as u32 / FACE_INSTANCE_SIZE,
			solid_quads: self.mc.get_pos(SOLID_QUADS_CURSOR) as u32 / FACE_INSTANCE_SIZE,
			solid_tris: self.mc.get_pos(SOLID_TRIS_CURSOR) as u32 / FACE_INSTANCE_SIZE,
		}
	}
	
	pub fn into_buffer(self) -> Box<[u8]> {
		let textured_quads = to_instance_range(self.mc.get_range(TEXTURED_QUADS_CURSOR));
		let textured_tris = to_instance_range(self.mc.get_range(TEXTURED_TRIS_CURSOR));
		let solid_quads = to_instance_range(self.mc.get_range(SOLID_QUADS_CURSOR));
		let solid_tris = to_instance_range(self.mc.get_range(SOLID_TRIS_CURSOR));
		println!("textured quads: {}", textured_quads.clone().count());
		println!("textured tris: {}", textured_tris.clone().count());
		println!("solid quads: {}", solid_quads.clone().count());
		println!("solid tris: {}", solid_tris.clone().count());
		self.mc.into_buffer()
	}
}
