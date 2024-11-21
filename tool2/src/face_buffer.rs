use crate::{as_bytes::ReinterpretAsBytes, multi_cursor::{MultiCursorBuffer, TypedWriter}};

//1 MB
const FACE_BUFFER_SIZE: usize = 1048576;

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

/**
Bits:  
`IIIIIIIIIIIIIIIITTTTTTTTTTTTTTTTFFFFFFFFFFFFFFFFAAAAAAAAAAAAAAAA`  
`A`: face array index (16 bits)  
`F`: face index (16 bits)  
`T`: transform index (16 bits)  
`I`: id (16 bits)
*/
#[repr(C)]
pub struct FaceInstance(u64);

impl FaceInstance {
	pub fn new(face_array_index: usize, face_index: usize, transform_index: usize, id: usize) -> Self {
		assert!(face_array_index < 65536);
		assert!(face_index < 65536);
		assert!(transform_index < 65536);
		assert!(id < 65536);
		Self(
			face_array_index as u64
			| ((face_index as u64) << 16)
			| ((transform_index as u64) << 32)
			| ((id as u64) << 48)
		)
	}
}

impl ReinterpretAsBytes for FaceInstance {}

#[derive(Clone, Copy, Debug)]
pub enum FaceType {
	TexturedQuad,
	TexturedTri,
	SolidQuad,
	SolidTri,
}

impl FaceType {
	fn cursor(self) -> usize {
		match self {
			Self::TexturedQuad => TEXTURED_QUADS_CURSOR,
			Self::TexturedTri => TEXTURED_TRIS_CURSOR,
			Self::SolidQuad => SOLID_QUADS_CURSOR,
			Self::SolidTri => SOLID_TRIS_CURSOR,
		}
	}
}

pub struct FaceBuffer {
	mc: MultiCursorBuffer,
}

impl FaceBuffer {
	pub fn new() -> Self {
		Self {
			mc: MultiCursorBuffer::new(
				FACE_BUFFER_SIZE,
				&[TEXTURED_QUADS_OFFSET, TEXTURED_TRIS_OFFSET, SOLID_QUADS_OFFSET, SOLID_TRIS_OFFSET],
			),
		}
	}
	
	pub fn get_writer(&mut self, face_type: FaceType) -> TypedWriter<FaceInstance> {
		self.mc.get_writer(face_type.cursor()).type_wrap()
	}
	
	pub fn into_buffer(self) -> Box<[u8]> {
		println!("textured quads: {}", self.mc.get_range(TEXTURED_QUADS_CURSOR).count() / size_of::<FaceInstance>());
		println!("textured tris: {}", self.mc.get_range(TEXTURED_TRIS_CURSOR).count() / size_of::<FaceInstance>());
		println!("solid quads: {}", self.mc.get_range(SOLID_QUADS_CURSOR).count() / size_of::<FaceInstance>());
		println!("solid tris: {}", self.mc.get_range(SOLID_TRIS_CURSOR).count() / size_of::<FaceInstance>());
		self.mc.into_buffer()
	}
}
