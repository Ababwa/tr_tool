use std::{borrow::Cow, collections::HashMap, fs::File, io::BufReader, ops::Range};
use glam_traits::glam::{ivec3, u16vec2, uvec2, IVec3, UVec2, Vec2, Vec3};
use tr_reader::tr4::{self, IMAGE_SIZE};
use crate::reinterpret;

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
				.map(|[x, y]| u16vec2(x, y + atlas_and_triangle.atlas_id() * IMAGE_SIZE as u16))
				.map(|v| v.as_vec2()),
			blend_mode: *blend_mode,
		})
		.collect()
}

#[repr(C)]
pub struct Vertex {
	pub pos: Vec3,
	pub tex: Vec2,
}

fn add_vertices<const N: usize>(
	opaque: &mut Vec<Vertex>,
	additive: &mut Vec<Vertex>,
	room_verts: &[Vec3],
	obj_texs: &[ObjTex],
	faces: &[tr4::RoomFace<N>],
	indices: &[usize],
) {
	for &tr4::RoomFace { texture_details, vertex_ids } in faces {
		let tex_id = texture_details.texture_id() as usize;
		let obj_tex = &obj_texs[tex_id];
		let vertex_list = if let tr4::BlendMode::Add = obj_tex.blend_mode {
			&mut *additive
		} else {
			&mut *opaque
		};
		let indices = if texture_details.double_sided() {
			Cow::Owned(indices.iter().chain(indices.iter().rev()).copied().collect())
		} else {
			Cow::Borrowed(indices)
		};
		for &i in indices.iter() {
			vertex_list.push(Vertex {
				pos: room_verts[vertex_ids[i] as usize],
				tex: obj_tex.vertices[i],
			})
		}
	}
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

fn build_geom(rooms: &[tr4::Room], obj_texs: &[ObjTex]) -> BuildGeomOutput {
	let mut vertices = vec![];
	let room_vertex_indices = rooms.iter().map(|room| {
			let room_verts = room.vertices
				.iter()
				.map(|tr4::RoomVertex { vertex, .. }| vertex.as_ivec3())
				.map(|IVec3 { x, y, z }| ivec3(x + room.x, y, z + room.z).as_vec3() / 1024.0)
				.collect::<Vec<_>>();
			let mut opaque = vec![];
			let mut additive = vec![];
			add_vertices(&mut opaque, &mut additive, &room_verts, obj_texs, &room.triangles, &[0, 1, 2]);
			add_vertices(&mut opaque, &mut additive, &room_verts, obj_texs, &room.quads, &[0, 1, 2, 0, 2, 3]);
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
		})
		.collect::<Vec<_>>();
	let mut static_room_indices = (0..rooms.len()).collect::<Vec<_>>();
	let mut flip_groups = HashMap::<u8, Vec<FlipRoom>>::new();
	for (index, room) in rooms.iter().enumerate() {
		if let Some(flip_index) = room.flip_room_id {
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

pub fn load_level(level_path: &str) -> LevelRenderData {
	let tr4::Level {
		images: tr4::Images { images32, .. },
		level_data: tr4::LevelData { object_textures, rooms, .. },
		..
	} = tr4::read_level(&mut BufReader::new(File::open(level_path).expect("failed to open file")))
		.expect("failed to read level");
	let object_textures = transform_object_textures(&object_textures);
	let BuildGeomOutput { vertices, room_vertex_indices, static_room_indices, flip_groups } = build_geom(&rooms, &object_textures);
	LevelRenderData {
		atlas_size: uvec2(IMAGE_SIZE as u32, (images32.len() * IMAGE_SIZE) as u32),
		atlas_data: unsafe { reinterpret::box_slice(images32) },//byte arrays to byte arrays
		vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	}
}
