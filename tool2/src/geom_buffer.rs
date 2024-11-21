use std::mem::size_of;
use glam::{I16Vec3, Mat4};
use tr_model::{tr1, tr2, tr3};
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, multi_cursor::MultiCursorBuffer, WrittenFaceArray};

//2 MB
pub const GEOM_BUFFER_SIZE: usize = 2097152;

//0..1048576, len: 1048576 (1/2 buffer)
const GEOM_CURSOR: usize = 0;
const GEOM_OFFSET: usize = 0;

//1048576..1310720, len: 262144 (1/8 buffer)
const OBJECT_TEXTURES_CURSOR: usize = 1;
const OBJECT_TEXTURES_OFFSET: usize = 1048576;

//1310720..1572864, len: 262144 (1/8 buffer)
const TRANSFORMS_CURSOR: usize = 2;
const TRANSFORMS_OFFSET: usize = 1310720;

//1572864..1835008, len: 262144 (1/8 buffer)
const SPRITE_TEXTURES_CURSOR: usize = 3;
const SPRITE_TEXTURES_OFFSET: usize = 1572864;

//1835008..2097152, len: 262144 (1/8 buffer)
const FACE_ARRAY_MAP_CURSOR: usize = 4;
const FACE_ARRAY_MAP_OFFSET: usize = 1835008;

pub trait Vertex {}

impl Vertex for I16Vec3 {}
impl Vertex for tr1::RoomVertex {}
impl Vertex for tr2::RoomVertex {}
impl Vertex for tr3::RoomVertex {}

pub trait Face {}

impl Face for tr1::RoomQuad {}
impl Face for tr1::RoomTri {}
impl Face for tr1::MeshTexturedQuad {}
impl Face for tr1::MeshTexturedTri {}
impl Face for tr1::MeshSolidQuad {}
impl Face for tr1::MeshSolidTri {}
impl Face for tr2::MeshSolidQuad {}
impl Face for tr2::MeshSolidTri {}
impl Face for tr3::RoomQuad {}
impl Face for tr3::RoomTri {}

pub struct GeomBuffer {
	mc: MultiCursorBuffer,
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
					FACE_ARRAY_MAP_OFFSET,
				],
			),
		}
	}
	
	pub fn write_object_textures(&mut self, object_textures: &[tr1::ObjectTexture]) {
		self.mc.get_writer(OBJECT_TEXTURES_CURSOR).write(object_textures.as_bytes()).unwrap();
	}
	
	pub fn write_sprite_textures(&mut self, sprite_textures: &[tr1::SpriteTexture]) {
		self.mc.get_writer(SPRITE_TEXTURES_CURSOR).write(sprite_textures.as_bytes()).unwrap();
	}
	
	pub fn write_vertex_array<V>(&mut self, vertices: &[V]) -> usize
	where V: Vertex + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.pos() / 2;
		geom_writer.write(&(size_of::<V>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(vertices.as_bytes()).unwrap();
		offset
	}
	
	pub fn write_face_array<'a, F>(&mut self, faces: &'a [F], vertex_array_offset: usize) -> WrittenFaceArray
	where F: Face + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.pos();
		geom_writer.write(&(vertex_array_offset as u32).to_le_bytes()).unwrap();
		geom_writer.write(&(size_of::<F>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(faces.as_bytes()).unwrap();
		let mut face_array_map_writer = self.mc.get_writer(FACE_ARRAY_MAP_CURSOR);
		let index = face_array_map_writer.size() / 4;
		face_array_map_writer.write(&(offset as u32 / 2).to_le_bytes()).unwrap();
		WrittenFaceArray { index, len: faces.len() }
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
		println!("FACE_MAP bytes: {}", self.mc.get_size(FACE_ARRAY_MAP_CURSOR));
		self.mc.into_buffer()
	}
}
