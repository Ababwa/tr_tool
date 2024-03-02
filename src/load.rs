use std::{collections::HashMap, f32::consts::TAU, fs::File, io::BufReader, ops::Range, slice};
use glam::{ivec3, u16vec2, uvec2, IVec3, Mat4, U16Vec3, UVec2, Vec2, Vec3};
use itertools::Itertools;
use tr_reader::tr4::{self, IMAGE_SIZE};
use shared::reinterpret;

/// TR texture coord units are 1/256 of a pixel.
/// Transform to whole pixel units by rounding to nearest.
fn transform_coord(a: u16) -> u16 {
	(a >> 8) + (((a & 255) + 128) >> 8)
}

struct ObjTex {
	vertices: [Vec2; 4],
	blend_mode: tr4::BlendMode,
}

fn transform_object_textures(object_textures: &[tr4::ObjectTexture]) -> Vec<ObjTex> {
	object_textures
		.iter()
		.map(|tr4::ObjectTexture { blend_mode, atlas_and_triangle, vertices, .. }| ObjTex {
			vertices: vertices
				.map(|v| v.to_array().map(transform_coord))
				.map(|[x, y]| u16vec2(x, y + atlas_and_triangle.atlas_index() * IMAGE_SIZE as u16))
				.map(|v| v.as_vec2()),
			blend_mode: *blend_mode,
		})
		.collect()
}

#[repr(C)]
pub struct Vertex {
	pub pos: Vec3,
	pub uv: Vec2,
}

struct Indices<'a> {
	single: &'a [usize],
	double: &'a [usize],
}

impl<'a> Indices<'a> {
	const fn new(indices: &'a [usize]) -> Self {
		Self {
			single: unsafe { slice::from_raw_parts(indices.as_ptr(), indices.len() / 2) },//const slice
			double: indices,
		}
	}
}

//ABC -> ABCCBA
macro_rules! concat_reverse {
	(; $($original:expr),*; $($reversed:expr),*) => {
		[$($original),* $(, $reversed)*]
	};
	($($head:expr)? $(, $tail:expr)* $(; $($original:expr),*; $($reversed:expr),*)?) => {
		concat_reverse!($($tail),*; $($($original,)*)? $($head)?; $($head)? $($(, $reversed)*)?)
	};
}

const TRI_INDICES: Indices = Indices::new(&concat_reverse!(0, 1, 2));
const QUAD_INDICES: Indices = Indices::new(&concat_reverse!(0, 1, 2, 0, 2, 3));

fn add_face<const N: usize>(
	vertex_list: &mut Vec<Vertex>,
	positions: &[Vec3],
	uvs: &[Vec2; 4],
	face: &tr4::Face<N>,
	indices: &Indices,
) {
	let indices = match face.texture_details.double_sided() {
		true => indices.double,
		false => indices.single,
	};
	for &i in indices.iter() {
		vertex_list.push(Vertex {
			pos: positions[face.vertex_indices[i] as usize],
			uv: uvs[i],
		})
	}
}

fn add_room_faces<const N: usize>(
	opaque: &mut Vec<Vertex>,
	additive: &mut Vec<Vertex>,
	obj_texs: &[ObjTex],
	positions: &[Vec3],
	faces: &[tr4::Face<N>],
	indices: &Indices,
) {
	for face in faces {
		let ObjTex { vertices: uvs, blend_mode } = &obj_texs[face.texture_details.texture_index() as usize];
		let vertex_list = match blend_mode {
			tr4::BlendMode::Add => &mut *additive,
			_ => &mut *opaque,
		};
		add_face(vertex_list, positions, uvs, face, indices);
	}
}

fn add_mesh_faces<const N: usize>(
	opaque: &mut Vec<Vertex>,
	additive: &mut Vec<Vertex>,
	obj_texs: &[ObjTex],
	positions: &[Vec3],
	mesh_faces: &[tr4::MeshFace<N>],
	indices: &Indices,
) {
	for mesh_face in mesh_faces {
		let ObjTex { vertices: uvs, blend_mode } = &obj_texs[mesh_face.face.texture_details.texture_index() as usize];
		let vertex_list = if mesh_face.effects.additive() {
			&mut *additive
		} else {
			match blend_mode {
				tr4::BlendMode::Add => &mut *additive,
				_ => &mut *opaque,
			}
		};
		add_face(vertex_list, positions, uvs, &mesh_face.face, indices);
	}
}

fn add_mesh(
	opaque: &mut Vec<Vertex>,
	additive: &mut Vec<Vertex>,
	obj_texs: &[ObjTex],
	transform: Mat4,
	mesh: &tr4::Mesh,
) {
	let mesh_verts = mesh.vertices.iter().map(|v| transform.transform_point3(v.as_vec3() / 1024.0)).collect::<Vec<_>>();
	add_mesh_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.triangles, &TRI_INDICES);
	add_mesh_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.quads, &QUAD_INDICES);
}

pub struct RoomVertexIndices {
	pub opaque: Range<u32>,
	pub additive: Range<u32>,
}

pub struct FlipRoom {
	pub original: usize,
	pub flipped: usize,
}

impl FlipRoom {
	pub fn get_room_index(&self, flipped: bool) -> usize {
		match flipped {
			true => self.flipped,
			false => self.original,
		}
	}
}

pub struct FlipGroup {
	pub label: u8,
	pub flip_rooms: Vec<FlipRoom>,
	pub flipped: bool,
}

impl FlipGroup {
	pub fn get_room_indices<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
		self.flip_rooms.iter().map(|flip_room| flip_room.get_room_index(self.flipped))
	}
}

struct BuildGeomOutput {
	vertices: Vec<Vertex>,
	room_vertex_indices: Vec<RoomVertexIndices>,
	static_room_indices: Vec<usize>,
	flip_groups: Vec<FlipGroup>,
}

fn get_rotation(rot: tr4::FrameRotation) -> Mat4 {
	fn t(r: u16, d: f32) -> f32 { r as f32 / d * TAU }
	match rot {
		tr4::FrameRotation::X(x) => Mat4::from_rotation_x(t(x, 4096.0)),
		tr4::FrameRotation::Y(y) => Mat4::from_rotation_y(t(y, 4096.0)),
		tr4::FrameRotation::Z(z) => Mat4::from_rotation_z(t(z, 4096.0)),
		tr4::FrameRotation::All(U16Vec3 { x, y, z }) =>
			Mat4::from_rotation_y(t(y, 1024.0)) *
			Mat4::from_rotation_x(t(x, 1024.0)) *
			Mat4::from_rotation_z(t(z, 1024.0)),
	}
}

fn build_geom(
	obj_texs: &[ObjTex],
	meshes: &tr4::Meshes,
	mesh_node_data: &tr4::MeshNodeData,
	frame_data: &tr4::FrameData,
	static_meshes: &HashMap<u16, tr4::StaticMesh>,
	models: &HashMap<u16, tr4::Model>,
	entities: &HashMap<usize, Vec<tr4::Entity>>,
	rooms: &[tr4::Room],
) -> BuildGeomOutput {
	let mut vertices = vec![];
	let room_vertex_indices = rooms.iter().enumerate().map(|(room_index, room)| {
		let room_verts = room.vertices
			.iter()
			.map(|tr4::RoomVertex { vertex, .. }| vertex.as_ivec3())
			.map(|IVec3 { x, y, z }| ivec3(x + room.x, y, z + room.z).as_vec3() / 1024.0)
			.collect::<Vec<_>>();
		let mut opaque = vec![];
		let mut additive = vec![];
		add_room_faces(&mut opaque, &mut additive, obj_texs, &room_verts, &room.triangles, &TRI_INDICES);
		add_room_faces(&mut opaque, &mut additive, obj_texs, &room_verts, &room.quads, &QUAD_INDICES);
		for room_static_mesh in room.room_static_meshes.iter() {
			let transform = Mat4::from_translation(room_static_mesh.pos.as_vec3() / 1024.0) * Mat4::from_rotation_y(room_static_mesh.rotation as f32 / 65536.0 * TAU);
			add_mesh(
				&mut opaque,
				&mut additive,
				obj_texs,
				transform,
				meshes.get_mesh(static_meshes[&room_static_mesh.static_mesh_id].mesh_id),
			);
		}
		for entity in entities.get(&room_index).map(|v| v.as_slice()).unwrap_or_default() {
			let model = &models[&entity.model_id];
			let frame = frame_data.get_frame(model.frame_byte_offset, model.num_meshes);
			let entity_transform = Mat4::from_translation(entity.pos.as_vec3() / 1024.0) * Mat4::from_rotation_y(entity.rotation as f32 / 65536.0 * TAU);
			let transform = Mat4::from_translation(frame.offset.as_vec3() / 1024.0) * get_rotation(frame.rotations[0]);
			add_mesh(&mut opaque, &mut additive, obj_texs, entity_transform * transform, meshes.get_mesh(model.mesh_id));
			let mut last_transform = transform;
			let mut parent_stack = vec![];
			for (index, mesh_node) in mesh_node_data.get_mesh_nodes(model.mesh_node_offset, model.num_meshes - 1).into_iter().enumerate() {
				let parent = match mesh_node.details.pop() {
					true => parent_stack.pop().unwrap_or_default(),
					false => last_transform,
				};
				if mesh_node.details.push() {
					parent_stack.push(parent);
				}
				let transform = parent * Mat4::from_translation(mesh_node.offset.as_vec3() / 1024.0) * get_rotation(frame.rotations[index + 1]);
				add_mesh(&mut opaque, &mut additive, obj_texs, entity_transform * transform, meshes.get_mesh(model.mesh_id + index as u16 + 1));
				last_transform = transform;
			}
		}
		let opaque_start = vertices.len();
		let opaque_end = opaque_start + opaque.len();
		let additive_start = opaque_end;
		let additive_end = additive_start + additive.len();
		vertices.extend(opaque);
		vertices.extend(additive);
		RoomVertexIndices {
			opaque: opaque_start as u32..opaque_end as u32,
			additive: additive_start as u32..additive_end as u32,
		}
	}).collect();
	let mut static_room_indices = (0..rooms.len()).collect::<Vec<_>>();
	let mut flip_groups = HashMap::<u8, Vec<FlipRoom>>::new();
	for (index, room) in rooms.iter().enumerate() {
		if let Some(flip_index) = room.flip_room_index {
			let flip_index = flip_index.get() as usize;
			static_room_indices.remove(static_room_indices.binary_search(&index).expect("no_flips missing index"));
			static_room_indices.remove(static_room_indices.binary_search(&flip_index).expect("no_flips missing flip index"));
			flip_groups.entry(room.flip_group).or_default().push(FlipRoom { original: index, flipped: flip_index });
		}
	}
	let mut flip_groups = flip_groups
		.into_iter()
		.map(|(label, flip_rooms)| FlipGroup { label, flip_rooms, flipped: false })
		.collect::<Vec<_>>();
	flip_groups.sort_by_key(|flip_group| flip_group.label);
	BuildGeomOutput {
		vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	}
}

pub struct LevelRenderData {
	pub atlas_size: UVec2,
	pub atlas_data: Box<[u8]>,
	pub vertices: Vec<Vertex>,
	pub room_vertex_indices: Vec<RoomVertexIndices>,
	pub static_room_indices: Vec<usize>,
	pub flip_groups: Vec<FlipGroup>,
}

pub fn load_level_render_data(level_path: &str) -> LevelRenderData {
	let tr4::Level {
		images: tr4::Images { images32, .. },
		level_data: tr4::LevelData {
			object_textures,
			meshes,
			mesh_node_data,
			frame_data,
			static_meshes,
			models,
			entities,
			rooms,
			..
		},
		..
	} = tr4::read_level(&mut BufReader::new(File::open(level_path).expect("failed to open file")))
		.expect("failed to read level");
	let static_meshes = static_meshes.into_vec().into_iter().map(|static_mesh| (static_mesh.id as u16, static_mesh)).collect();
	let models = models.into_vec().into_iter().map(|model| (model.id as u16, model)).collect();
	let entities = entities.into_vec().into_iter().into_group_map_by(|e| e.room_index as usize);
	let object_textures = transform_object_textures(&object_textures);
	let BuildGeomOutput {
		vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	} = build_geom(
		&object_textures,
		&meshes,
		&mesh_node_data,
		&frame_data,
		&static_meshes,
		&models,
		&entities,
		&rooms,
	);
	LevelRenderData {
		atlas_size: uvec2(IMAGE_SIZE as u32, (images32.len() * IMAGE_SIZE) as u32),
		atlas_data: unsafe { reinterpret::box_slice(images32) },//byte arrays to byte arrays
		vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	}
}
