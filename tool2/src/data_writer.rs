use std::ops::Range;
use tr_model::{tr1, tr3};
use crate::{
	as_bytes::ReinterpretAsBytes, face_buffer::{FaceBuffer, FaceInstance, FaceType},
	geom_buffer::{Face, GeomBuffer}, multi_cursor::TypedWriter, ObjectData, WrittenFaceArray, WrittenMesh,
};

pub trait RoomFace {
	fn double_sided(&self) -> bool;
}

impl RoomFace for tr1::RoomQuad { fn double_sided(&self) -> bool { false } }
impl RoomFace for tr1::RoomTri { fn double_sided(&self) -> bool { false } }
impl RoomFace for tr3::RoomQuad { fn double_sided(&self) -> bool { self.texture.double_sided() } }
impl RoomFace for tr3::RoomTri { fn double_sided(&self) -> bool { self.texture.double_sided() } }

pub struct DataWriter {
	pub geom_buffer: GeomBuffer,
	pub face_buffer: FaceBuffer,
	pub object_data: Vec<ObjectData>,
}

fn instantiate_face_array(
	object_data: &mut Vec<ObjectData>, face_writer: &mut TypedWriter<FaceInstance>,
	face_array: &WrittenFaceArray, transform_index: usize, object_data_maker: impl Fn(usize) -> ObjectData,
) -> Range<u32> {
	let start = face_writer.pos() as u32;
	for face_index in 0..face_array.len {
		let fi = FaceInstance::new(face_array.index, face_index, transform_index, object_data.len());
		face_writer.write(&fi).unwrap();
		object_data.push(object_data_maker(face_index));
	}
	let end = face_writer.pos() as u32;
	start..end
}

pub struct MeshFaceInstanceRanges {
	pub textured_quads: Range<u32>,
	pub textured_tris: Range<u32>,
	pub solid_quads: Range<u32>,
	pub solid_tris: Range<u32>,
}

pub struct RoomFaceInstanceOffsets {
	pub start: u32,
	pub flipped_start: u32,
	pub end: u32,
}

impl RoomFaceInstanceOffsets {
	pub fn original(&self) -> Range<u32> {
		self.start..self.flipped_start
	}
	
	pub fn flipped(&self) -> Range<u32> {
		self.flipped_start..self.end
	}
}

impl DataWriter {
	pub fn instantiate_mesh(
		&mut self, mesh: &WrittenMesh, transform_index: usize,
		object_data_maker: impl Fn(FaceType, usize) -> ObjectData,
	) -> MeshFaceInstanceRanges {
		MeshFaceInstanceRanges {
			textured_quads: instantiate_face_array(
				&mut self.object_data, &mut self.face_buffer.get_writer(FaceType::TexturedQuad),
				&mesh.textured_quads, transform_index,
				|face_index| object_data_maker(FaceType::TexturedQuad, face_index),
			),
			textured_tris: instantiate_face_array(
				&mut self.object_data, &mut self.face_buffer.get_writer(FaceType::TexturedTri),
				&mesh.textured_tris, transform_index,
				|face_index| object_data_maker(FaceType::TexturedTri, face_index),
			),
			solid_quads: instantiate_face_array(
				&mut self.object_data, &mut self.face_buffer.get_writer(FaceType::SolidQuad),
				&mesh.solid_quads, transform_index,
				|face_index| object_data_maker(FaceType::SolidQuad, face_index),
			),
			solid_tris: instantiate_face_array(
				&mut self.object_data, &mut self.face_buffer.get_writer(FaceType::SolidTri),
				&mesh.solid_tris, transform_index,
				|face_index| object_data_maker(FaceType::SolidTri, face_index),
			),
		}
	}
	
	pub fn write_room_face_array<F: RoomFace + Face + ReinterpretAsBytes>(
		&mut self, vertex_array_offset: usize, face_type: FaceType, faces: &[F], transform_index: usize,
		object_data_maker: impl Fn(usize) -> ObjectData,
	) -> RoomFaceInstanceOffsets {
		let face_array = self.geom_buffer.write_face_array(faces, vertex_array_offset);
		let mut face_writer = self.face_buffer.get_writer(face_type);
		let object_data_offset = self.object_data.len();
		let Range { start, end: flipped_start } = instantiate_face_array(
			&mut self.object_data, &mut face_writer, &face_array, transform_index, object_data_maker,
		);
		for (face_index, face) in faces.iter().enumerate() {
			if face.double_sided() {
				let fi = FaceInstance::new(
					face_array.index, face_index, transform_index, self.object_data.len(),
				);
				face_writer.write(&fi).unwrap();
				self.object_data.push(ObjectData::flipped(object_data_offset + face_index));
			}
		}
		let end = face_writer.pos() as u32;
		RoomFaceInstanceOffsets { start, flipped_start, end }
	}
}
