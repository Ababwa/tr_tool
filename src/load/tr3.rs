use std::io::{Read, Result};
use glam::{uvec2, Mat4};
use shared::reinterpret;
use tr_reader::model::{self as tr, tr3};
use super::{
	add_colored_faces, add_textured_faces, get_level_render_data,
	ColoredVertex, LevelRenderData, LocalTrVersion, ObjTex, TexturedVertex,
};

impl LocalTrVersion for tr3::Tr3 {
	const FRAME_SINGLE_ROT_DIVISOR: f32 = 1024.0;
	const COLORED: bool = true;
	type Mesh = tr3::Mesh;
	type RoomLight = tr3::Light;
	type ObjectTexture = tr3::ObjectTexture;
	
	fn flip_group(_room: &tr::Room<Self::RoomLight>) -> u8 { 0 }
	
	fn get_obj_tex_details(obj_tex: &Self::ObjectTexture) -> (
			tr::BlendMode,
			tr::ObjectTextureAtlasAndTriangle,
			[glam::U16Vec2; 4],
		) {
		(obj_tex.blend_mode, obj_tex.atlas_and_triangle, obj_tex.vertices)
	}
	
	fn add_mesh(
		opaque: &mut Vec<TexturedVertex>,
		additive: &mut Vec<TexturedVertex>,
		colored: &mut Vec<ColoredVertex>,
		obj_texs: &[ObjTex],
		transform: Mat4,
		mesh: &Self::Mesh,
	) {
		let mesh_verts = mesh
			.vertices
			.iter()
			.map(|v| transform.transform_point3(v.as_vec3() / 1024.0))
			.collect::<Vec<_>>();
		add_textured_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.textured_tris);
		add_textured_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.textured_quads);
		add_colored_faces(colored, &mesh_verts, &mesh.colored_tris);
		add_colored_faces(colored, &mesh_verts, &mesh.colored_quads);
	}
}

fn to_bgra(images: &[[u16; tr::NUM_PIXELS]]) -> Box<[u8]> {
	let mut vec = Vec::with_capacity(images.len() * tr::NUM_PIXELS * 4);
	for image in images {
		for &pixel in image {
			vec.push(((pixel & 31) << 3) as u8);
			vec.push(((pixel & 992) >> 2) as u8);
			vec.push(((pixel & 31744) >> 7) as u8);
			vec.push(((pixel >> 15) * 255) as u8);
		}
	}
	vec.into_boxed_slice()
}

pub fn load_level_render_data<R: Read>(reader: &mut R) -> Result<LevelRenderData> {
	let tr3::Level {
		palette4,
		images: tr3::Images { images16, .. },
		rooms,
		meshes,
		mesh_node_data,
		frame_data,
		models,
		static_meshes,
		object_textures,
		entities,
		..
	} = tr3::read_level(reader)?;
	Ok(get_level_render_data::<tr3::Tr3>(
		Some(unsafe { reinterpret::boxx(palette4) }),//struct of bytes to bytes
		uvec2(tr::IMAGE_SIZE as u32, (images16.len() * tr::IMAGE_SIZE) as u32),
		to_bgra(&images16),
		&rooms,
		&meshes,
		&mesh_node_data,
		&frame_data,
		&models,
		&static_meshes,
		&object_textures,
		&entities,
	))
}
