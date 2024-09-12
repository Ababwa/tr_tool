use std::io::{Read, Result};
use glam::{uvec2, Mat4};
use tr_model::{shared::{Color3, Mesh, FRAME_SINGLE_ROT_MASK_TR123, IMAGE_SIZE, NUM_PIXELS, PALETTE_SIZE}, tr1::{read_level, Images, Level, Tr1}};
use super::{add_mesh_tr123, get_level_render_data, SolidVertex, LevelRenderData, ObjTex, TexturedVertex, TrVersionExt, FRAME_SINGLE_ROT_DIVISOR_TR123};

impl TrVersionExt for Tr1 {
	const FRAME_SINGLE_ROT_MASK: u16 = FRAME_SINGLE_ROT_MASK_TR123;
	const FRAME_SINGLE_ROT_DIVISOR: f32 = FRAME_SINGLE_ROT_DIVISOR_TR123;
	
	fn flip_group(_room_extra: &Self::RoomExtra) -> u8 { 0 }
	
	fn add_mesh(
		opaque: &mut Vec<TexturedVertex>,
		additive: &mut Vec<TexturedVertex>,
		solid: &mut Vec<SolidVertex>,
		obj_texs: &[ObjTex],
		transform: Mat4,
		mesh: &Mesh<Self::MeshComponent>,
	) {
		add_mesh_tr123(opaque, additive, solid, obj_texs, transform, mesh);
	}
}

fn to_4(palette: &[Color3; PALETTE_SIZE]) -> Box<[u8; PALETTE_SIZE * 4]> {
	let mut bytes = Vec::with_capacity(PALETTE_SIZE * 4);
	for color in palette {
		bytes.push(color.r);
		bytes.push(color.g);
		bytes.push(color.b);
		bytes.push(255);
	}
	bytes.into_boxed_slice().try_into().ok().unwrap()//exactly 1024
}

fn to_bgra(palette: &[Color3; PALETTE_SIZE], palette_images: &[[u8; NUM_PIXELS]]) -> Box<[u8]> {
	let mut bytes = Vec::with_capacity(palette_images.len() * NUM_PIXELS * 4);
	for image in palette_images {
		for &index in image {
			let color = palette[index as usize];
			bytes.push(color.b);
			bytes.push(color.g);
			bytes.push(color.r);
			bytes.push(255);
		}
	}
	bytes.into_boxed_slice()
}

pub fn load_level_render_data<R: Read>(reader: &mut R) -> Result<LevelRenderData> {
	let Level {
		images: Images(palette_images),
		rooms,
		meshes,
		mesh_node_data,
		frame_data,
		models,
		static_meshes,
		sprite_textures,
		sprite_sequences,
		object_textures,
		entities,
		palette,
		..
	} = read_level(reader)?;
	Ok(get_level_render_data::<Tr1>(
		Some(to_4(&palette)),
		uvec2(IMAGE_SIZE as u32, (palette_images.len() * IMAGE_SIZE) as u32),
		to_bgra(&palette, &palette_images),
		&rooms,
		&meshes,
		&mesh_node_data,
		&frame_data,
		&models,
		&static_meshes,
		&sprite_textures,
		&sprite_sequences,
		&object_textures,
		&entities,
	))
}
