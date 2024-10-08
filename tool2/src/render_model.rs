use tr_model::tr1;
use wgpu::{Buffer, BufferUsages, Device};
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, make, MESH_VERTEX_SIZE, QUAD_FACE_SIZE, TRI_FACE_SIZE};

#[derive(Clone, Copy)]
pub enum ModelRef<'a> {
	Model(&'a tr1::Model),
	SpriteSequence(&'a tr1::SpriteSequence),
}

#[repr(C)]
pub struct FaceInstance {
	pub face_offset: u32,
	pub vertices_offset: u32,
	pub transform_index: u16,
	pub face_size: u8,
	pub vertex_size: u8,
}

impl ReinterpretAsBytes for FaceInstance {}

#[derive(Default)]
pub struct Faces {
	pub textured_quads: Vec<FaceInstance>,
	pub textured_tris: Vec<FaceInstance>,
	pub solid_quads: Vec<FaceInstance>,
	pub solid_tris: Vec<FaceInstance>,
}

#[derive(Clone, Copy)]
pub struct FaceArray {
	pub offset: u32,
	pub len: u32,
}

pub struct Mesh {
	pub vertices_offset: u32,
	pub textured_quads: FaceArray,
	pub textured_tris: FaceArray,
	pub solid_quads: FaceArray,
	pub solid_tris: FaceArray,
}

pub struct FaceBuffer {
	pub buffer: Buffer,
	pub len: u32,
}

pub struct FaceBuffers {
	pub textured_quads: FaceBuffer,
	pub textured_tris: FaceBuffer,
	pub solid_quads: FaceBuffer,
	pub solid_tris: FaceBuffer,
}

impl Faces {
	pub fn add_mesh(&mut self, mesh: &Mesh, transform_index: u16) {
		for (face_list, face_array, face_size) in [
			(&mut self.textured_quads, mesh.textured_quads, QUAD_FACE_SIZE),
			(&mut self.textured_tris, mesh.textured_tris, TRI_FACE_SIZE),
			(&mut self.solid_quads, mesh.solid_quads, QUAD_FACE_SIZE),
			(&mut self.solid_tris, mesh.solid_tris, TRI_FACE_SIZE),
		] {
			for face_index in 0..face_array.len {
				face_list.push(FaceInstance {
					face_offset: face_array.offset + face_index * face_size as u32,
					vertices_offset: mesh.vertices_offset,
					transform_index,
					face_size,
					vertex_size: MESH_VERTEX_SIZE,
				});
			}
		}
	}
	
	pub fn into_buffers(self, device: &Device) -> FaceBuffers {
		println!("num textured_quads: {}", self.textured_quads.len());
		println!("num textured_tris: {}", self.textured_tris.len());
		println!("num solid_quads: {}", self.solid_quads.len());
		println!("num solid_tris: {}", self.solid_tris.len());
		let [
			textured_quads,
			textured_tris,
			solid_quads,
			solid_tris,
		] = [
			self.textured_quads,
			self.textured_tris,
			self.solid_quads,
			self.solid_tris,
		].map(|face_list| {
			FaceBuffer {
				buffer: make::buffer(device, face_list.as_bytes(), BufferUsages::VERTEX),
				len: face_list.len() as u32,
			}
		});
		FaceBuffers {
			textured_quads,
			textured_tris,
			solid_quads,
			solid_tris,
		}
	}
}
