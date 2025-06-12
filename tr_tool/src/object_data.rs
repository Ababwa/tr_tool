use glam::{I16Vec3, IVec3, Vec3};
use glam_traits::GVec;
use tr_model::{tr1, tr2};
use crate::{
	tr_traits::{
		Entity, Level, Mesh, Model, ObjectTexture, Room, RoomFace, RoomStaticMesh, RoomVertex, SolidFace, TexturedFace
	},
	InteractPixel,
};

#[derive(Clone, Copy, Debug)]
pub enum PolyType {
	Quad,
	Tri,
}

#[derive(Clone, Copy, Debug)]
pub enum MeshFaceType {
	TexturedQuad,
	TexturedTri,
	SolidQuad,
	SolidTri,
}

#[derive(Clone, Copy, Debug)]
pub enum ObjectData {
	RoomFace {
		room_index: u16,
		layer_index: u16,
		face_type: PolyType,
		face_index: u16,
	},
	RoomStaticMeshFace {
		room_index: u16,
		room_static_mesh_index: u16,
		face_type: MeshFaceType,
		face_index: u16,
	},
	RoomSprite {
		room_index: u16,
		sprite_index: u16,
	},
	EntityMeshFace {
		entity_index: u16,
		mesh_index: u16,
		face_type: MeshFaceType,
		face_index: u16,
	},
	EntitySprite {
		entity_index: u16,
	},
	Reverse {
		object_data_index: u32,
	},
}

fn cw(verts: &[Vec3], dim: usize) -> i8 {
	let d1 = (dim + 1) % 3;
	let d2 = (dim + 2) % 3;
	let a1 = verts[1][d1] - verts[0][d1];
	let a2 = verts[1][d2] - verts[0][d2];
	let b1 = verts[2][d1] - verts[0][d1];
	let b2 = verts[2][d2] - verts[0][d2];
	((a1 * b2 - a2 * b1) as i8).signum()
}

pub fn print_object_data<L: Level>(level: &L, object_data: &[ObjectData], index: InteractPixel) {
	println!("object data index: {}", index);
	let data = match object_data.get(index as usize) {
		Some(&data) => data,
		None => {
			println!("out of bounds");
			return;
		},
	};
	println!("{:?}", data);
	let data = match data {
		ObjectData::Reverse { object_data_index } => {
			let data = object_data[object_data_index as usize];
			println!("{:?}", data);
			data
		},
		data => data,
	};
	let mesh_face = match data {
		ObjectData::RoomFace { room_index, layer_index, face_type, face_index } => {
			fn face_data<const N: usize, T: RoomFace<N>, L: Level>(faces: &[T], face_index: u16, vertices: &[<<L as Level>::Room as Room>::RoomVertex]) -> (u16, bool) {
				let face = &faces[face_index as usize];
				let verts = face.vertex_indices().map(|i| vertices[i as usize].pos().as_vec());
				for (index, vert) in verts.iter().enumerate() {
					println!("vert {}: {}", index, vert);
				}
				let [x, y, z] = [0, 1, 2].map(|d| cw(&verts, d));
				println!("cw: {}, {}, {}", x, y, z);
				(face.object_texture_index(), face.double_sided())
			}
			let room = &level.rooms()[room_index as usize];
			let layer = room.layers().nth(layer_index as usize).unwrap();//unwrap: proven in level parse
			let (object_texture_index, double_sided) = match face_type {
				PolyType::Quad => face_data::<4, _, L>(&layer.quads, face_index, layer.vertices),
				PolyType::Tri => face_data::<3, _, L>(&layer.tris, face_index, layer.vertices),
			};
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("double sided: {}", double_sided);
			println!("blend mode: {}", object_texture.blend_mode());
			None
		},
		ObjectData::RoomStaticMeshFace { room_index, room_static_mesh_index, face_type, face_index } => {
			let room = &level.rooms()[room_index as usize];
			let room_static_mesh = &room.room_static_meshes()[room_static_mesh_index as usize];
			let static_mesh_id = room_static_mesh.static_mesh_id();
			//unwrap: proven in level parse
			let static_mesh = level
				.static_meshes()
				.iter()
				.find(|static_mesh| static_mesh.id as u16 == static_mesh_id)
				.unwrap();
			let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
			println!("static mesh id: {}", static_mesh_id);
			Some((mesh_offset, face_type, face_index))
		},
		ObjectData::RoomSprite { room_index, sprite_index } => {
			_ = (room_index, sprite_index);
			None
		},
		ObjectData::EntityMeshFace { entity_index, mesh_index, face_type, face_index } => {
			let model_id = level.entities()[entity_index as usize].model_id();
			//unwrap: proven in level parse
			let model = level.models().iter().find(|model| model.id() as u16 == model_id).unwrap();
			let mesh_offset = level.mesh_offsets()[(model.mesh_offset_index() + mesh_index) as usize];
			println!("model id: {}", model_id);
			Some((mesh_offset, face_type, face_index))
		},
		ObjectData::EntitySprite { entity_index } => {
			_ = entity_index;
			None
		},
		ObjectData::Reverse { .. } => panic!("reverse points to reverse"),
	};
	if let Some((mesh_offset, face_type, face_index)) = mesh_face {
		println!("mesh offset: {}", mesh_offset);
		let mesh = level.get_mesh(mesh_offset);
		let (object_texture_index, color_index_24bit, color_index_32bit) = match face_type {
			MeshFaceType::TexturedQuad => {
				(Some(mesh.textured_quads()[face_index as usize].object_texture_index()), None, None)
			},
			MeshFaceType::TexturedTri => {
				(Some(mesh.textured_tris()[face_index as usize].object_texture_index()), None, None)
			},
			MeshFaceType::SolidQuad => {
				let quad = &mesh.solid_quads()[face_index as usize];
				(None, Some(quad.color_index_24bit()), quad.color_index_32bit())
			},
			MeshFaceType::SolidTri => {
				let tri = &mesh.solid_tris()[face_index as usize];
				(None, Some(tri.color_index_24bit()), tri.color_index_32bit())
			},
		};
		if let Some(object_texture_index) = object_texture_index {
			let object_texture = &level.object_textures()[object_texture_index as usize];
			println!("blend mode: {}", object_texture.blend_mode());
		}
		if let (Some(color_index), Some(palette)) = (color_index_24bit, level.palette_24bit()) {
			let tr1::Color24Bit { r, g, b } = palette[color_index as usize];
			let [r, g, b] = [r, g, b].map(|c| (c << 2) as u32);
			let color = (r << 16) | (g << 8) | b;
			println!("color 24 bit: #{:06X}", color);
		}
		if let (Some(color_index), Some(palette)) = (color_index_32bit, level.palette_32bit()) {
			let &tr2::Color32BitRgb { r, g, b } = &palette[color_index as usize];
			let [r, g, b] = [r, g, b].map(|c| c as u32);
			let color = (r << 16) | (g << 8) | b;
			println!("color 32 bit: #{:06X}", color);
		}
	}
}
