use std::io::{Read, Result};
use glam::{uvec2, Mat4, Vec3};
use tr_reader::model::{self as tr, tr4};
use super::{
	add_textured_face, get_level_render_data,
	SolidVertex, LevelRenderData, TrVersionExt, ObjTex, TexturedVertex,
};

fn add_mesh_faces<const N: usize>(
	opaque: &mut Vec<TexturedVertex>,
	additive: &mut Vec<TexturedVertex>,
	obj_texs: &[ObjTex],
	positions: &[Vec3],
	mesh_faces: &[tr::MeshFace<N>],
) {
	for mesh_face in mesh_faces {
		let ObjTex { vertices: uvs, blend_mode } = &obj_texs[mesh_face.face.texture_details.texture_index() as usize];
		let vertex_list = if mesh_face.effects.additive() {
			&mut *additive
		} else {
			match blend_mode {
				tr::BlendMode::Add => &mut *additive,
				_ => &mut *opaque,
			}
		};
		add_textured_face(vertex_list, positions, uvs, &mesh_face.face);
	}
}

impl TrVersionExt for tr4::Tr4 {
	const FRAME_SINGLE_ROT_DIVISOR: f32 = 4096.0;
	type Mesh = tr::Mesh<tr::MeshComponentTr45>;
	type RoomExtra = tr4::RoomExtra;
	
	fn flip_group(room_extra: &Self::RoomExtra) -> u8 { room_extra.flip_group }
	
	fn add_mesh(
		opaque: &mut Vec<TexturedVertex>,
		additive: &mut Vec<TexturedVertex>,
		_solid: &mut Vec<SolidVertex>,
		obj_texs: &[ObjTex],
		transform: Mat4,
		mesh: &Self::Mesh,
	) {
		let mesh_verts = mesh.vertices.iter().map(|v| transform.transform_point3(v.as_vec3() / 1024.0)).collect::<Vec<_>>();
		add_mesh_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.component.tris);
		add_mesh_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.component.quads);
	}
}

pub fn load_level_render_data<R: Read>(reader: &mut R) -> Result<LevelRenderData> {
	let tr4::Level {
		images: tr4::Images { images32, .. },
		level_data: tr4::LevelData {
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
		},
		..
	} = tr4::read_level(reader)?;
	Ok(get_level_render_data::<tr4::Tr4, _, _, _, _, _, _>(
		None,
		uvec2(tr::IMAGE_SIZE as u32, (images32.len() * tr::IMAGE_SIZE) as u32),
		unsafe { reinterpret::box_slice(images32) },//primitive arrays to byte array
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
