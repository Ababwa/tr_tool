use std::{mem::MaybeUninit, ops::Range};
use shared::alloc;
use tr_model::tr3;
use crate::{
	as_bytes::{ReinterpretAsBytes, ToBytes}, geom_buffer::GeomBuffer,
	tr_traits::{Level, MeshFace, MeshFaceType, RoomFace, RoomFaceType, TexturedFace}, ObjectData,
	WrittenFaceArray, WrittenMesh,
};

const NUM_FACES: usize = 65536;

#[repr(C)]
#[derive(Clone, Copy)]
struct FaceInstance {
	face_array_index: u16,
	face_index: u16,
	transform_index: u16,
	object_data_index: u16,
}

impl FaceInstance {
	fn new(
		face_array_index: usize, face_index: usize, transform_index: usize, object_data_index: usize,
	) -> Self {
		Self {
			face_array_index: face_array_index.try_into().unwrap(),
			face_index: face_index.try_into().unwrap(),
			transform_index: transform_index.try_into().unwrap(),
			object_data_index: object_data_index.try_into().unwrap(),
		}
	}
}

impl ReinterpretAsBytes for FaceInstance {}

pub struct MeshTexturedFaceOffsets {
	pub opaque: u32,
	pub additive: u32,
	pub end: u32,
}

impl MeshTexturedFaceOffsets {
	pub fn opaque(&self) -> Range<u32> {
		self.opaque..self.additive
	}
	
	pub fn additive(&self) -> Range<u32> {
		self.additive..self.end
	}
}

pub struct MeshFaceOffsets {
	pub textured_quads: MeshTexturedFaceOffsets,
	pub textured_tris: MeshTexturedFaceOffsets,
	pub solid_quads: Range<u32>,
	pub solid_tris: Range<u32>,
}

pub struct RoomFaceOffsets {
	pub original: u32,
	pub additive: u32,
	pub reverse: u32,
	pub additive_reverse: u32,
	pub end: u32,
}

impl RoomFaceOffsets {
	pub fn original(&self) -> Range<u32> {
		self.original..self.additive
	}
	
	pub fn additive(&self) -> Range<u32> {
		self.additive..self.reverse
	}
	
	pub fn reverse(&self) -> Range<u32> {
		self.reverse..self.additive_reverse
	}
	
	pub fn reverse_additive(&self) -> Range<u32> {
		self.additive_reverse..self.end
	}
}

pub struct Results {
	pub geom_buffer: Box<[u8]>,
	pub face_buffer: Box<[u8]>,
	pub object_data: Vec<ObjectData>,
}

pub struct DataWriter {
	pub geom_buffer: GeomBuffer,
	face_buffer: Box<[MaybeUninit<FaceInstance>; NUM_FACES]>,
	object_data: Box<[MaybeUninit<ObjectData>; NUM_FACES]>,
	num_written_faces: usize,
}

impl DataWriter {
	pub fn new(geom_buffer: GeomBuffer) -> Self {
		Self {
			geom_buffer,
			face_buffer: unsafe { alloc::val().assume_init() },
			object_data: unsafe { alloc::val().assume_init() },
			num_written_faces: 0,
		}
	}
	
	pub fn write_room_face_array<L, F, O>(
		&mut self, level: &L, vertex_array_offset: usize, faces: &[F], transform_index: usize,
		object_data_maker: O,
	) -> RoomFaceOffsets
	where L: Level, F: RoomFace, O: Fn(RoomFaceType, usize) -> ObjectData {
		let face_array_index = self.geom_buffer.write_face_array(faces, vertex_array_offset);
		let mut double_sided = Vec::with_capacity(faces.len());
		let original = self.num_written_faces;
		let mut additive = original + faces.len();
		let reverse = additive;
		for (face_index, face) in faces.iter().enumerate() {
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode;
			let is_additive = blend_mode == tr3::blend_mode::ADD;
			let index = if is_additive {
				additive -= 1;
				additive
			} else {
				face_index + additive - faces.len()
			};
			let object_data = object_data_maker(F::TYPE, face_index);
			let face_instance = FaceInstance::new(face_array_index, face_index, transform_index, index);
			self.object_data[index].write(object_data);
			self.face_buffer[index].write(face_instance);
			if face.double_sided() {
				double_sided.push((face_instance, is_additive));
			}
		}
		let mut additive_reverse = reverse + double_sided.len();
		let end = additive_reverse;
		for (ds_index, &(mut face_instance, is_additive)) in double_sided.iter().enumerate() {
			let index = if is_additive {
				additive_reverse -= 1;
				additive_reverse
			} else {
				ds_index + additive_reverse - double_sided.len()
			};
			let object_data = ObjectData::Reverse { object_data_index: face_instance.object_data_index };
			face_instance.object_data_index = index as u16;
			self.object_data[index].write(object_data);
			self.face_buffer[index].write(face_instance);
		}
		self.num_written_faces = end;
		let original = original as u32;
		let additive = additive as u32;
		let reverse = reverse as u32;
		let additive_reverse = additive_reverse as u32;
		let end = end as u32;
		RoomFaceOffsets { original, additive, reverse, additive_reverse, end }
	}
	
	fn mesh_textured_face_array<L, F, O>(
		&mut self, level: &L, face_array: &WrittenFaceArray<F>, transform_index: usize, object_data_maker: O,
	) -> MeshTexturedFaceOffsets
	where L: Level, F: TexturedFace + MeshFace, O: Fn(MeshFaceType, usize) -> ObjectData {
		let opaque = self.num_written_faces;
		let mut additive = opaque + face_array.faces.len();
		let end = additive;
		for (face_index, face) in face_array.faces.iter().enumerate() {
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode;
			let index = if blend_mode == tr3::blend_mode::ADD {
				additive -= 1;
				additive
			} else {
				face_index + additive - face_array.faces.len()
			};
			let object_data = object_data_maker(F::TYPE, face_index);
			let face_instance = FaceInstance::new(face_array.index, face_index, transform_index, index);
			self.object_data[index].write(object_data);
			self.face_buffer[index].write(face_instance);
		}
		self.num_written_faces = end;
		let opaque = opaque as u32;
		let additive = additive as u32;
		let end = end as u32;
		MeshTexturedFaceOffsets { opaque, additive, end }
	}
	
	fn mesh_solid_face_array<F, O>(
		&mut self, face_array: &WrittenFaceArray<F>, transform_index: usize, object_data_maker: O,
	) -> Range<u32>
	where F: MeshFace, O: Fn(MeshFaceType, usize) -> ObjectData {
		let start = self.num_written_faces;
		let end = start + face_array.faces.len();
		for face_index in 0..face_array.faces.len() {
			let index = start + face_index;
			let object_data = object_data_maker(F::TYPE, face_index);
			let face_instance = FaceInstance::new(face_array.index, face_index, transform_index, index);
			self.object_data[index].write(object_data);
			self.face_buffer[index].write(face_instance);
		}
		self.num_written_faces = end;
		let start = start as u32;
		let end = end as u32;
		start..end
	}
	
	pub fn instantiate_mesh<L, O>(
		&mut self, level: &L, mesh: &WrittenMesh<L>, transform_index: usize, object_data_maker: O,
	) -> MeshFaceOffsets
	where L: Level, O: Fn(MeshFaceType, usize) -> ObjectData {
		MeshFaceOffsets {
			textured_quads: self.mesh_textured_face_array(level, &mesh.textured_quads, transform_index, &object_data_maker),
			textured_tris: self.mesh_textured_face_array(level, &mesh.textured_tris, transform_index, &object_data_maker),
			solid_quads: self.mesh_solid_face_array(&mesh.solid_quads, transform_index, &object_data_maker),
			solid_tris: self.mesh_solid_face_array(&mesh.solid_tris, transform_index, &object_data_maker),
		}
	}
	
	pub fn done(self) -> Results {
		Results {
			geom_buffer: self.geom_buffer.into_buffer(), face_buffer: self.face_buffer.to_bytes(),
			object_data: unsafe {
				Vec::from_raw_parts(
					Box::into_raw(self.object_data).cast(), self.num_written_faces, NUM_FACES,
				)
			},
		}
	}
}
