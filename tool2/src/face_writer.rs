use std::ops::Range;
use tr_model::tr1;
use crate::{data_writer::FaceArrayRef, multi_cursor::MultiCursorBuffer, FaceRanges, MeshFaceArrayRefs};

//128 KB
const FACES_SIZE: usize = 131072;

//0..65536, len: 65536 (1/2 buffer)
const TEXTURED_QUADS_CURSOR: usize = 0;
const TEXTURED_QUADS_OFFSET: usize = 0;

//65536..98304, len: 32768 (1/4 buffer)
const TEXTURED_TRIS_CURSOR: usize = 1;
const TEXTURED_TRIS_OFFSET: usize = 65536;

//98304..114688, len: 16384 (1/8 buffer)
const SOLID_QUADS_CURSOR: usize = 2;
const SOLID_QUADS_OFFSET: usize = 98304;

//114688..131072, len: 16384 (1/8 buffer)
const SOLID_TRIS_CURSOR: usize = 3;
const SOLID_TRIS_OFFSET: usize = 114688;

fn face_instance(face_array_index: u32, face_index: u32, transform_index: u32) -> u32 {
	assert!(face_array_index < 1024, "face_array_index: {}", face_array_index);
	assert!(face_index < 1024, "face_index: {}", face_index);
	assert!(transform_index < 1024, "transform_index: {}", transform_index);
	face_array_index | (face_index << 10) | (transform_index << 20)
}

fn to_instance_range(byte_range: Range<usize>) -> Range<u32> {
	byte_range.start as u32 / 4..byte_range.end as u32 / 4
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
			face_type_writer.write(
				&face_instance(face_array_ref.index, face_index, transform_index).to_le_bytes(),
			);
		}
	}
	
	pub fn write_mesh(&mut self, mesh: &MeshFaceArrayRefs, transform_index: u32) {
		self.write_face_instance_array(mesh.textured_quads, transform_index);
		self.write_face_instance_array(mesh.textured_tris, transform_index);
		self.write_face_instance_array(mesh.solid_quads, transform_index);
		self.write_face_instance_array(mesh.solid_tris, transform_index);
	}
	
	pub fn into_ranges(mut self) -> (FaceRanges, Box<[u8]>) {
		let textured_quads = to_instance_range(self.mc.get_writer(TEXTURED_QUADS_CURSOR).range());
		let textured_tris = to_instance_range(self.mc.get_writer(TEXTURED_TRIS_CURSOR).range());
		let solid_quads = to_instance_range(self.mc.get_writer(SOLID_QUADS_CURSOR).range());
		let solid_tris = to_instance_range(self.mc.get_writer(SOLID_TRIS_CURSOR).range());
		println!("num textured_quads: {}", textured_quads.clone().count());
		println!("num textured_tris: {}", textured_tris.clone().count());
		println!("num solid_quads: {}", solid_quads.clone().count());
		println!("num solid_tris: {}", solid_tris.clone().count());
		(FaceRanges { textured_quads, textured_tris, solid_quads, solid_tris }, self.mc.into_buffer())
	}
}
