use std::{iter, mem::size_of};
use glam::Mat4;
use tr_model::tr1;
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, tr_traits::Face};

/// 4 MB
pub const GEOM_BUFFER_SIZE: usize = 4194304;

pub struct Output {
	pub data_buffer: Box<[u8; GEOM_BUFFER_SIZE]>,
	/// Offset of transforms in 16-byte units.
	pub transforms_offset: u32,
	/// Offset of face array offsets in 4-byte units.
	pub face_array_offsets_offset: u32,
	/// Offset of object textures in 2-byte units.
	pub object_textures_offset: u32,
	/// Offset of sprite textures in 2-byte units.
	pub sprite_textures_offset: u32,
}

pub struct GeomBuffer {
	geom: Vec<u8>,
	face_array_offsets: Vec<u32>,
	transforms: Vec<Mat4>,
}

impl GeomBuffer {
	pub fn new() -> Self {
		Self {
			geom: vec![],
			face_array_offsets: vec![],
			transforms: vec![],
		}
	}
	
	/**
	Writes the following record to the geometry buffer 4-aligned:  
	`SSSS[V..]`  
	`S`: Vertex size in 2-byte units.  
	`V`: Verices. Always a multiple of 2 bytes.  
	Returns offset in 4-byte units.
	*/
	pub fn write_vertex_array<V: ReinterpretAsBytes>(&mut self, vertices: &[V]) -> u32 {
		let offset = self.geom.len();//always multiple of 2
		let padding = offset % 4;//pad to 4-align
		self.geom.reserve(padding + 4 + size_of_val(vertices));
		self.geom.extend(iter::repeat_n(0, padding));
		self.geom.extend_from_slice((size_of::<V>() as u32 / 2).as_bytes());
		self.geom.extend_from_slice(vertices.as_bytes());
		(offset + padding) as u32 / 4
	}
	
	/**
	Writes the following record to the geometry buffer 4-aligned:  
	`VVVVSSTT[F..]`  
	`V`: Vertex array offset in 4-byte units.  
	`S`: Face size in 2-byte units.  
	`T`: Texture offset in 2-byte units.  
	`F`: Faces. Always a multiple of 2 bytes.  
	Returns index of face array.
	*/
	pub fn write_face_array<F: Face + ReinterpretAsBytes>(&mut self, faces: &[F], vertex_array_offset: u32) -> u16 {
		let texture_offset = faces.get(0).map(|face| face.vertex_indices().len()).unwrap_or_default() as u16;
		let index = self.face_array_offsets.len().try_into().unwrap();
		let offset = self.geom.len();//always multiple of 2
		let padding = offset % 4;//pad to 4-align
		self.geom.reserve(padding + 8 + size_of_val(faces));
		self.geom.extend(iter::repeat_n(0, padding));
		self.geom.extend_from_slice(vertex_array_offset.as_bytes());
		self.geom.extend_from_slice((size_of::<F>() as u16 / 2).as_bytes());
		self.geom.extend_from_slice(texture_offset.as_bytes());
		self.geom.extend_from_slice(faces.as_bytes());
		self.face_array_offsets.push((offset + padding) as u32 / 4);
		index
	}
	
	pub fn write_transform(&mut self, transform: &Mat4) -> u16 {
		let index = self.transforms.len().try_into().unwrap();
		self.transforms.push(*transform);
		index
	}
	
	/**
	Creates the following record:  
	`[G..][P..][T..][F..][O..][S..]`  
	`G`: Geometry data. Always a multiple of 2 bytes.  
	`P`: Padding to align-16.  
	`T`: Transform matrices. Always a multiple of 64 bytes.  
	`F`: Face array offsets. Always a multiple of 4 bytes.  
	`O`: Object textures. Always a multiple of 2 bytes.  
	`S`: Sprite textures. Always a multiple of 2 bytes.
	*/
	pub fn into_buffer<O: ReinterpretAsBytes>(
		self, object_textures: &[O], sprite_textures: &[tr1::SpriteTexture],
	) -> Output {
		let geom_bytes = self.geom.len();
		let transforms_bytes = size_of_val(&*self.transforms);
		let face_array_offsets_bytes = size_of_val(&*self.face_array_offsets);
		let object_textures_bytes = size_of_val(object_textures);
		let sprite_textures_bytes = size_of_val(sprite_textures);
		
		println!("geom_bytes: {}", geom_bytes);
		println!("transforms_bytes: {}", transforms_bytes);
		println!("face_array_offsets_bytes: {}", face_array_offsets_bytes);
		println!("object_textures_bytes: {}", object_textures_bytes);
		println!("sprite_textures_bytes: {}", sprite_textures_bytes);
		
		let padding = (16 - (geom_bytes % 16)) % 16;
		let transforms_offset = geom_bytes + padding;
		let face_array_offsets_offset = transforms_offset + transforms_bytes;
		let object_textures_offset = face_array_offsets_offset + face_array_offsets_bytes;
		let sprite_textures_offset = object_textures_offset + object_textures_bytes;
		let size = sprite_textures_offset + sprite_textures_bytes;
		
		println!("total: {}", size);
		assert!(size < GEOM_BUFFER_SIZE);
		
		let mut data_buffer = unsafe { Box::<[u8; GEOM_BUFFER_SIZE]>::new_uninit().assume_init() };
		data_buffer[..geom_bytes].copy_from_slice(&self.geom);
		data_buffer[transforms_offset..][..transforms_bytes].copy_from_slice(self.transforms.as_bytes());
		data_buffer[face_array_offsets_offset..][..face_array_offsets_bytes].copy_from_slice(self.face_array_offsets.as_bytes());
		data_buffer[object_textures_offset..][..object_textures_bytes].copy_from_slice(object_textures.as_bytes());
		data_buffer[sprite_textures_offset..][..sprite_textures_bytes].copy_from_slice(sprite_textures.as_bytes());
		
		Output {
			data_buffer,
			transforms_offset: transforms_offset as u32 / 16,
			face_array_offsets_offset: face_array_offsets_offset as u32 / 4,
			object_textures_offset: object_textures_offset as u32 / 2,
			sprite_textures_offset: sprite_textures_offset as u32 / 2,
		}
	}
}
