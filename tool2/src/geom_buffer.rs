use std::mem::size_of;
use glam::Mat4;
use tr_model::tr1;
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, multi_cursor::MultiCursorBuffer, tr_traits::Face, PolyType};

//2 MB
pub const GEOM_BUFFER_SIZE: usize = 2097152;

//0..1048576, len: 1048576 (1/2 buffer)
const GEOM_CURSOR: usize = 0;
const GEOM_OFFSET: usize = 0;

//1048576..1572864, len: 524288 (1/4 buffer)
const OBJECT_TEXTURES_CURSOR: usize = 1;
const OBJECT_TEXTURES_OFFSET: usize = 1048576;

//1572864..1835008, len: 262144 (1/8 buffer)
const TRANSFORMS_CURSOR: usize = 2;
const TRANSFORMS_OFFSET: usize = 1572864;

//1835008..2097152, len: 262144 (1/8 buffer)
const SPRITE_TEXTURES_CURSOR: usize = 3;
const SPRITE_TEXTURES_OFFSET: usize = 1835008;

pub struct GeomBuffer {
	mc: MultiCursorBuffer,
}

fn texture_offset(poly_type: PolyType) -> u8 {
	match poly_type {
		PolyType::Quad => 4,
		PolyType::Tri => 3,
	}
}

impl GeomBuffer {
	pub fn new() -> Self {
		Self {
			mc: MultiCursorBuffer::new(
				GEOM_BUFFER_SIZE,
				&[
					GEOM_OFFSET,
					OBJECT_TEXTURES_OFFSET,
					TRANSFORMS_OFFSET,
					SPRITE_TEXTURES_OFFSET,
				],
			),
		}
	}
	
	pub fn write_object_textures<O: ReinterpretAsBytes>(&mut self, object_textures: &[O]) {
		let mut object_textures_writer = self.mc.get_writer(OBJECT_TEXTURES_CURSOR);
		object_textures_writer.write(&(size_of::<O>() as u16 / 2).to_le_bytes()).unwrap();
		object_textures_writer.write(object_textures.as_bytes()).unwrap();
	}
	
	pub fn write_sprite_textures(&mut self, sprite_textures: &[tr1::SpriteTexture]) {
		self.mc.get_writer(SPRITE_TEXTURES_CURSOR).write(sprite_textures.as_bytes()).unwrap();
	}
	
	pub fn write_vertex_array<V: ReinterpretAsBytes>(&mut self, vertices: &[V]) -> usize {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		geom_writer.align(16).unwrap();
		let offset = geom_writer.pos() / 16;
		geom_writer.write(&(size_of::<V>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(vertices.as_bytes()).unwrap();
		offset
	}
	
	pub fn write_face_array<F: Face>(&mut self, faces: &[F], vertex_array_offset: usize) -> usize {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		geom_writer.align(16).unwrap();
		let offset = geom_writer.pos() / 16;
		geom_writer.write(&u16::try_from(vertex_array_offset).unwrap().to_le_bytes()).unwrap();
		geom_writer.write(&[size_of::<F>() as u8 / 2, texture_offset(F::POLY_TYPE)]).unwrap();
		geom_writer.write(faces.as_bytes()).unwrap();
		offset
	}
	
	pub fn write_transform(&mut self, transform: &Mat4) -> usize {
		let mut transforms_writer = self.mc.get_writer(TRANSFORMS_CURSOR);
		let index = transforms_writer.size() / size_of::<Mat4>();
		transforms_writer.write(transform.as_bytes()).unwrap();
		index
	}
	
	pub fn into_buffer(self) -> Box<[u8]> {
		println!("GEOM bytes: {}", self.mc.get_size(GEOM_CURSOR));
		println!("OBJECT_TEXTURES bytes: {}", self.mc.get_size(OBJECT_TEXTURES_CURSOR));
		println!("TRANSFORM bytes: {}", self.mc.get_size(TRANSFORMS_CURSOR));
		println!("SPRITE_TEXTURES bytes: {}", self.mc.get_size(SPRITE_TEXTURES_CURSOR));
		self.mc.into_buffer()
	}
}
