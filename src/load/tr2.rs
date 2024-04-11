use std::io::{Read, Result};
use glam::{uvec2, Mat4};
use tr_reader::model::{self as tr, tr2};
use super::{add_mesh_tr123, get_level_render_data, to_bgra, SolidVertex, LevelRenderData, ObjTex, TexturedVertex, TrVersionExt, FRAME_SINGLE_ROT_DIVISOR_TR123};

impl TrVersionExt for tr2::Tr2 {
	const FRAME_SINGLE_ROT_DIVISOR: f32 = FRAME_SINGLE_ROT_DIVISOR_TR123;
	type Mesh = tr::Mesh<tr::MeshComponentTr123>;
	type RoomExtra = ();
	
	fn flip_group(_room_extra: &Self::RoomExtra) -> u8 { 0 }
	
	fn add_mesh(
		opaque: &mut Vec<TexturedVertex>,
		additive: &mut Vec<TexturedVertex>,
		solid: &mut Vec<SolidVertex>,
		obj_texs: &[ObjTex],
		transform: Mat4,
		mesh: &Self::Mesh,
	) {
		add_mesh_tr123(opaque, additive, solid, obj_texs, transform, mesh);
	}
}

pub fn load_level_render_data<R: Read>(reader: &mut R) -> Result<LevelRenderData> {
	let tr2::Level {
		palette4,
		images: tr::Images { images16, .. },
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
	} = tr2::read_level(reader)?;
	Ok(get_level_render_data::<tr2::Tr2, _, _, _, _, _, _>(
		Some(unsafe { reinterpret::boxx(palette4) }),//struct of bytes to bytes
		uvec2(tr::IMAGE_SIZE as u32, (images16.len() * tr::IMAGE_SIZE) as u32),
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
