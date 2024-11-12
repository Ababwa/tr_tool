use std::{marker::PhantomData, ops::Range};
use tr_model::tr1;
use crate::{data_writer::FaceArrayRef, multi_cursor::MultiCursorBuffer, version_traits::{Level, Mesh}, FaceOffsets, MeshFaceArrayRefs};

//1 MB
const FACES_SIZE: usize = 1048576;

//0..524288, len: 524288 (1/2 buffer)
const TEXTURED_QUADS_CURSOR: usize = 0;
const TEXTURED_QUADS_OFFSET: usize = 0;

//524288..786432, len: 262144 (1/4 buffer)
const TEXTURED_TRIS_CURSOR: usize = 1;
const TEXTURED_TRIS_OFFSET: usize = 524288;

//786432..917504, len: 131072 (1/8 buffer)
const SOLID_QUADS_CURSOR: usize = 2;
const SOLID_QUADS_OFFSET: usize = 786432;

//917504..1048576, len: 131072 (1/8 buffer)
const SOLID_TRIS_CURSOR: usize = 3;
const SOLID_TRIS_OFFSET: usize = 917504;

const FACE_INSTANCE_SIZE: u32 = 8;

/*
face instance:
0000000000000000TTTTTTTTTTTTTTTTFFFFFFFFFFFFFFFFAAAAAAAAAAAAAAAA
A: face array index (16 bits)
F: face index (16 bits)
T: transform index (16 bits)
*/
fn face_instance(face_array_index: u32, face_index: u32, transform_index: u32, id: u32) -> u64 {
	assert!(face_array_index < 65536, "face_array_index: {}", face_array_index);
	assert!(face_index < 65536, "face_index: {}", face_index);
	assert!(transform_index < 65536, "transform_index: {}", transform_index);
	assert!(id < 65536, "id: {}", id);
	face_array_index as u64 |
	((face_index as u64) << 16) |
	((transform_index as u64) << 32) |
	((id as u64) << 48)
}

fn to_instance_range(byte_range: Range<usize>) -> Range<u32> {
	byte_range.start as u32 / FACE_INSTANCE_SIZE..byte_range.end as u32 / FACE_INSTANCE_SIZE
}

mod private {
	pub trait Face<L, D> { const CURSOR: usize; }
}

use private::Face;

impl<L: Level> Face<L, [(); 0]> for tr1::RoomQuad { const CURSOR: usize = TEXTURED_QUADS_CURSOR; }
impl<L: Level> Face<L, [(); 0]> for tr1::RoomTri { const CURSOR: usize = TEXTURED_TRIS_CURSOR; }
impl<L: Level> Face<L, [(); 0]> for tr1::MeshTexturedQuad { const CURSOR: usize = TEXTURED_QUADS_CURSOR; }
impl<L: Level> Face<L, [(); 0]> for tr1::MeshTexturedTri { const CURSOR: usize = TEXTURED_TRIS_CURSOR; }
impl<L: Level> Face<L, [(); 1]> for <L::Mesh<'_> as Mesh>::SolidQuad { const CURSOR: usize = SOLID_QUADS_CURSOR; }
impl<L: Level> Face<L, [(); 2]> for <L::Mesh<'_> as Mesh>::SolidTri { const CURSOR: usize = SOLID_TRIS_CURSOR; }
// impl Face for tr1::MeshSolidQuad { const CURSOR: usize = SOLID_QUADS_CURSOR; }
// impl Face for tr1::MeshSolidTri { const CURSOR: usize = SOLID_TRIS_CURSOR; }

pub struct FaceWriter<L> {
	mc: MultiCursorBuffer,
	u: PhantomData<L>,
}

impl<L: Level> FaceWriter<L> {
	pub fn new() -> Self {
		Self {
			mc: MultiCursorBuffer::new(
				FACES_SIZE,
				&[TEXTURED_QUADS_OFFSET, TEXTURED_TRIS_OFFSET, SOLID_QUADS_OFFSET, SOLID_TRIS_OFFSET],
			),
			u: PhantomData,
		}
	}
	
	pub fn write_face_instance_array<F, D>(
		&mut self, face_array_ref: FaceArrayRef<F>, transform_index: u32, id: u32,
	) where F: Face<L, D> {
		let mut face_type_writer = self.mc.get_writer(F::CURSOR);
		for face_index in 0..face_array_ref.len {
			if let Err(e) = face_type_writer.write(
				&face_instance(face_array_ref.index, face_index, transform_index, id).to_le_bytes(),
			) {
				panic!("write face instance fail: type: {}, msg: {}", F::CURSOR, e);
			}
		}
	}
	
	pub fn write_mesh<SolidQuad, D1, SolidTri, D2>(
		&mut self, mesh: &MeshFaceArrayRefs<SolidQuad, SolidTri>, transform_index: u32, id: u32,
	) where SolidQuad: Face<L, D1> + Copy, SolidTri: Face<L, D2> + Copy {
		self.write_face_instance_array(mesh.textured_quads, transform_index, id);
		self.write_face_instance_array(mesh.textured_tris, transform_index, id);
		self.write_face_instance_array(mesh.solid_quads, transform_index, id);
		self.write_face_instance_array(mesh.solid_tris, transform_index, id);
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
