use std::io::{Read, Result};
use glam::{uvec2, Mat4};
use tr_model::{shared::{ImagesTr23, Mesh, FRAME_SINGLE_ROT_MASK_TR123, IMAGE_SIZE}, tr3::{read_level, Level, Tr3}};
use super::{add_mesh_tr123, get_level_render_data, to_bgra, SolidVertex, LevelRenderData, TrVersionExt, ObjTex, TexturedVertex, FRAME_SINGLE_ROT_DIVISOR_TR123};

impl TrVersionExt for Tr3 {
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

pub fn load_level_render_data<R: Read>(reader: &mut R) -> Result<LevelRenderData> {
	let Level {
		palette4,
		images: ImagesTr23 { images16, .. },
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
		..
	} = read_level(reader)?;
	Ok(get_level_render_data::<Tr3>(
		Some(unsafe { reinterpret::boxx(palette4) }),//struct of bytes to bytes
		uvec2(IMAGE_SIZE as u32, (images16.len() * IMAGE_SIZE) as u32),
		to_bgra(&images16),
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
