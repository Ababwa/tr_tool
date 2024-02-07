use std::{fs::File, io::BufReader};
use glam_traits::glam::{u16vec2, U16Vec2, Vec2};
use tr_reader::{tr4::{self, IMG_DIM}, Readable};
use crate::{geom::{MinMax, PosSize, VecMinMax}, packer};

const PADDING: u16 = 4;

/// TR texture coord units are 1/256 of a pixel.
/// Transform to whole pixel units by rounding to nearest.
fn coord_transform(a: u16) -> u16 {
	(a >> 8) + (((a & 255) + 128) >> 8)
}

fn pixel_offset(width: usize, pos: U16Vec2) -> usize {
	(pos.y as usize * width + pos.x as usize) * 4
}

struct ImgView<T> {
	data: T,
	width: usize,
	offset: U16Vec2,
}

impl<T> ImgView<T> {
	fn new(data: T, width: usize, offset: U16Vec2) -> Self {
		Self { data, width, offset }
	}
}

impl<'a> ImgView<&'a [u8]> {
	fn get_pixel(&self, pos: U16Vec2) -> [u8; 4] {
		let offset = pixel_offset(self.width, self.offset + pos);
		let p = self.data[offset..offset + 4].as_ptr();
		unsafe { *(p as *const _) }
	}
}

impl<'a> ImgView<&'a mut [u8]> {
	fn put_pixel(&mut self, pos: U16Vec2, pixel: [u8; 4]) {
		let offset = pixel_offset(self.width, self.offset + pos);
		let p = self.data[offset..offset + 4].as_mut_ptr();
		unsafe { *(p as *mut _) = pixel }
	}
}

struct Block<T> {
	atlas: u16,
	rect: T,
	indices: Vec<usize>,
}

fn insert_index(blocks: &mut Vec<Block<MinMax<U16Vec2>>>, atlas: u16, rect: MinMax<U16Vec2>, index: usize) {
	for block in blocks.iter_mut() {
		if atlas == block.atlas {
			if block.rect.contains(&rect) {
				return block.indices.push(index);
			} else if rect.contains(&block.rect) {
				block.rect = rect;
				return block.indices.push(index);
			}
		}
	}
	blocks.push(Block { atlas, rect, indices: vec![index] });
}

/// Pack blocks with padding to create a new atlas.
/// Copy image data from old atlas to new.
/// Transform texture coords to normalized.
fn build_new_atlas(old_atlas: &[u8], blocks: Vec<Block<PosSize<U16Vec2>>>, mut texture_coords: Vec<[U16Vec2; 4]>) -> (U16Vec2, Vec<u8>, Vec<[Vec2; 4]>) {
	let (new_pos, new_atlas_size) = packer::pack(
		blocks
		.iter()
		.map(|block| block.rect.size + PADDING * 2)
	);
	let mut new_atlas = vec![0u8; new_atlas_size.as_uvec2().element_product() as usize * 4];
	for (Block { atlas, rect: PosSize { pos, size }, indices }, new_pos) in blocks.into_iter().zip(new_pos) {
		let delta = (new_pos + PADDING).as_i16vec2() - pos.as_i16vec2();
		for index in indices {
			for v in &mut texture_coords[index] {
				*v = v.wrapping_add_signed(delta);
			}
		}
		let src = ImgView::new(old_atlas, IMG_DIM, pos + atlas * IMG_DIM as u16 * U16Vec2::Y);
		let mut dest = ImgView::new(new_atlas.as_mut_slice(), new_atlas_size.x as usize, new_pos);
		for x in 0..size.x {//copy texture
			for y in 0..size.y {
				dest.put_pixel(PADDING + u16vec2(x, y), src.get_pixel(u16vec2(x, y)));
			}
		}
		for i in 0..2 {//edges
			let inv = 1 - U16Vec2::AXES[i];
			for j in 0..size[i] {
				let p = j * U16Vec2::AXES[i];
				let min_pixel = src.get_pixel(p);
				let max_pixel = src.get_pixel(p + inv * (size - 1));
				for k in 0..PADDING {
					dest.put_pixel(p + PADDING * U16Vec2::AXES[i] + k * inv, min_pixel);
					dest.put_pixel(p + PADDING + inv * (k + size), max_pixel);
				}
			}
		}
		for i in 0..2 {//corners
			for j in 0..2 {
				let pixel = src.get_pixel(u16vec2(i, j) * (size - 1));
				for x in 0..PADDING {
					for y in 0..PADDING {
						dest.put_pixel(u16vec2(i, j) * (PADDING + size) + u16vec2(x, y), pixel);
					}
				}
			}
		}
	}
	let atlas_size_f = new_atlas_size.as_vec2();
	let texture_coords = texture_coords.into_iter().map(|verts| verts.map(|v| v.as_vec2() / atlas_size_f)).collect();
	(new_atlas_size, new_atlas, texture_coords)
}

/// Extract texture coords from object textures.
/// Transform texture coord units to whole pixels.
/// Generate "blocks", rects in the old atlas that contain one or more textures.
fn get_blocks(object_textures: Box<[tr4::ObjectTexture]>) -> (Vec<Block<PosSize<U16Vec2>>>, Vec<[U16Vec2; 4]>) {
	let mut blocks = vec![];
	let texture_coords = object_textures
		.to_vec()//to get values
		.into_iter()
		.enumerate()
		.map(|(index, tr4::ObjectTexture { atlas_and_triangle, vertices, .. })| {
			let vertices = vertices.map(|v| U16Vec2::from(v.to_array().map(coord_transform)));
			let num = if atlas_and_triangle.triangle() { 3 } else { 4 };
			let rect = vertices[1..num].iter().copied().fold(MinMax::new(vertices[0]), MinMax::extend);
			insert_index(&mut blocks, atlas_and_triangle.atlas_id(), rect, index);
			vertices
		})
		.collect::<Vec<_>>();
	let blocks = blocks
		.into_iter()
		.map(|Block { atlas, rect, indices }| Block { atlas, rect: PosSize::from(rect), indices })
		.collect::<Vec<_>>();
	(blocks, texture_coords)
}

fn flatten<T, const N: usize>(data: &[[T; N]]) -> &[T] {
	unsafe { std::slice::from_raw_parts(data.as_ptr() as *const T, data.len() * N) }
}

pub struct LevelRenderData {
	pub atlas_size: U16Vec2,
	pub atlas_data: Vec<u8>,
	pub texture_coords: Vec<[Vec2; 4]>,
	pub rooms: Box<[tr4::Room]>,
}

pub fn load_level(level_path: &str) -> LevelRenderData {
	let tr4::Level {
		images: tr4::Images { images32, .. },
		level_data: tr4::LevelData { object_textures, rooms, .. },
		..
	} = tr4::Level::read(&mut BufReader::new(File::open(level_path).expect("failed to open file")))
		.expect("failed to read level");
	let (blocks, texture_coords) = get_blocks(object_textures);
	let (atlas_size, atlas_data, texture_coords) = build_new_atlas(flatten(&images32), blocks, texture_coords);
	LevelRenderData { atlas_size, atlas_data, texture_coords, rooms }
}
