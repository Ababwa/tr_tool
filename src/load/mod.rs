mod tr2;
mod tr3;
mod tr4;

use std::{collections::HashMap, f32::consts::TAU, fs::File, io::{BufReader, Result, Seek}, ops::Range, slice};
use byteorder::{ReadBytesExt, LE};
use glam::{i16vec2, ivec3, u16vec2, IVec3, Mat4, U16Vec2, U16Vec3, UVec2, Vec2, Vec3};
use glam_traits::ext::select;
use itertools::Itertools;
use tr_reader::model::{self as tr, IMAGE_SIZE};

const WORLD_COORD_SCALE: f32 = 1.0 / 1024.0;
const FRAME_SINGLE_ROT_DIVISOR_TR123: f32 = 1024.0;

#[repr(C)]
pub struct TexturedVertex {
	pub pos: Vec3,
	pub uv: Vec2,
}

#[repr(C)]
pub struct SolidVertex {
	pub pos: Vec3,
	pub color_index: u32,
}

#[repr(C)]
pub struct SpriteVertex {
	pub pos: Vec3,
	pub uv: Vec2,
	pub offset2d: Vec2,
}

pub struct RoomVertexIndices {
	pub opaque: Range<u32>,
	pub additive: Range<u32>,
	pub solid: Range<u32>,
	pub sprite: Range<u32>,
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

struct ObjTex {
	vertices: [Vec2; 4],
	blend_mode: tr::BlendMode,
}

struct SprTexVert {
	uv: Vec2,
	world_offset: Vec2,
}

/// TR texture coord units are 1/256 of a pixel.
/// Transform to whole pixel units by rounding to nearest.
fn transform_coord(a: u16) -> u16 {
	(a >> 8) + (((a & 255) + 128) >> 8)
}

fn transform_object_textures<D, C>(object_textures: &[tr::ObjectTexture<D, C>]) -> Vec<ObjTex> {
	object_textures.iter().map(|tr::ObjectTexture { blend_mode, atlas_and_triangle, vertices, .. }| ObjTex {
		vertices: vertices
			.map(|v| v.to_array().map(transform_coord))
			.map(|[x, y]| u16vec2(x, y + atlas_and_triangle.atlas_index() * tr::IMAGE_SIZE as u16))
			.map(|v| v.as_vec2()),
		blend_mode: *blend_mode,
	}).collect()
}

const CLOCKWISE_SQUARE: [U16Vec2; 4] = [u16vec2(0, 0), u16vec2(1, 0), u16vec2(1, 1), u16vec2(0, 1)];

fn transform_sprite_textures(sprite_textures: &[tr::SpriteTexture]) -> Vec<[SprTexVert; 4]> {
	sprite_textures.iter().map(|sprite_texture| {
		let pos = sprite_texture.pos.as_u16vec2() + u16vec2(0, sprite_texture.atlas_index * IMAGE_SIZE as u16);
		let size = (sprite_texture.size - 255) / 256;
		CLOCKWISE_SQUARE.map(|v| SprTexVert {
			uv: (pos + size * v).as_vec2(),
			world_offset: (select(&sprite_texture.world_bounds, &v) * i16vec2(1, -1)).as_vec2() * WORLD_COORD_SCALE,
		})
	}).collect()
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

/// A..Z -> A..ZZ..A
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

fn add_textured_face<const N: usize>(
	vertex_list: &mut Vec<TexturedVertex>,
	positions: &[Vec3],
	uvs: &[Vec2; 4],
	face: &tr::Face<N, tr::TexturedFaceDetails>,
) {
	let indices = match N {
		3 => TRI_INDICES,
		4 => QUAD_INDICES,
		_ => unreachable!(),
	};
	let indices = match face.texture_details.double_sided() {
		true => indices.double,
		false => indices.single,
	};
	for &i in indices {
		vertex_list.push(TexturedVertex {
			pos: positions[face.vertex_indices[i] as usize],
			uv: uvs[i],
		})
	}
}

fn add_textured_faces<const N: usize>(
	opaque: &mut Vec<TexturedVertex>,
	additive: &mut Vec<TexturedVertex>,
	object_textures: &[ObjTex],
	positions: &[Vec3],
	faces: &[tr::Face<N, tr::TexturedFaceDetails>],
) {
	for face in faces {
		let ObjTex { vertices: uvs, blend_mode } = &object_textures[face.texture_details.texture_index() as usize];
		let vertex_list = match blend_mode {
			tr::BlendMode::Add => &mut *additive,
			_ => &mut *opaque,
		};
		add_textured_face(vertex_list, positions, uvs, face);
	}
}

fn add_solid_faces<const N: usize>(
	solid: &mut Vec<SolidVertex>,
	positions: &[Vec3],
	faces: &[tr::Face<N, tr::SolidFaceDetails>],
) {
	let indices = match N {
		3 => TRI_INDICES,
		4 => QUAD_INDICES,
		_ => unreachable!(),
	}.single;
	for face in faces {
		for &i in indices {
			solid.push(SolidVertex {
				pos: positions[face.vertex_indices[i] as usize],
				color_index: face.texture_details.palette4_index as u32,
			});
		}
	}
}

fn add_mesh_tr123(
	opaque: &mut Vec<TexturedVertex>,
	additive: &mut Vec<TexturedVertex>,
	solid: &mut Vec<SolidVertex>,
	obj_texs: &[ObjTex],
	transform: Mat4,
	mesh: &tr::Mesh<tr::MeshComponentTr123>,
) {
	let mesh_verts = mesh
		.vertices
		.iter()
		.map(|v| transform.transform_point3(v.as_vec3() * WORLD_COORD_SCALE))
		.collect::<Vec<_>>();
	add_textured_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.component.textured_tris);
	add_textured_faces(opaque, additive, obj_texs, &mesh_verts, &mesh.component.textured_quads);
	add_solid_faces(solid, &mesh_verts, &mesh.component.solid_tris);
	add_solid_faces(solid, &mesh_verts, &mesh.component.solid_quads);
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

trait TrVersionExt: tr::TrVersion {
	const FRAME_SINGLE_ROT_DIVISOR: f32;
	type RoomExtra;
	type Mesh;
	
	fn flip_group(room: &Self::RoomExtra) -> u8;
	
	fn add_mesh(
		opaque: &mut Vec<TexturedVertex>,
		additive: &mut Vec<TexturedVertex>,
		solid: &mut Vec<SolidVertex>,
		object_textures: &[ObjTex],
		transform: Mat4,
		mesh: &Self::Mesh,
	);
}

fn get_rotation<T: TrVersionExt>(rot: tr::FrameRotation) -> Mat4 {
	fn t(r: u16, d: f32) -> f32 { r as f32 / d * TAU }
	match rot {
		tr::FrameRotation::X(x) => Mat4::from_rotation_x(t(x, T::FRAME_SINGLE_ROT_DIVISOR)),
		tr::FrameRotation::Y(y) => Mat4::from_rotation_y(t(y, T::FRAME_SINGLE_ROT_DIVISOR)),
		tr::FrameRotation::Z(z) => Mat4::from_rotation_z(t(z, T::FRAME_SINGLE_ROT_DIVISOR)),
		tr::FrameRotation::All(U16Vec3 { x, y, z }) =>
			Mat4::from_rotation_y(t(y, 1024.0)) *
			Mat4::from_rotation_x(t(x, 1024.0)) *
			Mat4::from_rotation_z(t(z, 1024.0)),
	}
}

pub struct SolidData {
	pub palette: Box<[u8; tr::PALETTE_SIZE * 4]>,
	pub solid_vertices: Vec<SolidVertex>,
}

pub struct LevelRenderData {
	pub atlas_size: UVec2,
	pub atlas_data: Box<[u8]>,
	pub textured_vertices: Vec<TexturedVertex>,
	pub solid: Option<SolidData>,
	pub sprite_vertices: Vec<SpriteVertex>,
	pub room_vertex_indices: Vec<RoomVertexIndices>,
	pub static_room_indices: Vec<usize>,
	pub flip_groups: Vec<FlipGroup>,
}

fn get_level_render_data<T: TrVersionExt, Rv, Ra, Rl, Od, Oc, Ec>(
	palette: Option<Box<[u8; tr::PALETTE_SIZE * 4]>>,
	atlas_size: UVec2,
	atlas_data: Box<[u8]>,
	rooms: &[tr::Room<Rv, Ra, Rl, T::RoomExtra>],
	meshes: &tr::Meshes<T::Mesh>,
	mesh_node_data: &tr::MeshNodeData,
	frame_data: &tr::FrameData,
	models: &[tr::Model],
	static_meshes: &[tr::StaticMesh],
	sprite_textures: &[tr::SpriteTexture],
	sprite_sequences: &[tr::SpriteSequence],
	object_textures: &[tr::ObjectTexture<Od, Oc>],
	entities: &[tr::Entity<Ec>],
) -> LevelRenderData {
	let models = models.iter().map(|model| (model.id as u16, model)).collect::<HashMap<_, _>>();
	let sprite_textures = transform_sprite_textures(sprite_textures);
	let static_meshes = static_meshes.iter().map(|static_mesh| (static_mesh.id as u16, static_mesh)).collect::<HashMap<_, _>>();
	let sprite_sequences = sprite_sequences.iter().map(|sprite_sequence| (sprite_sequence.id as u16, sprite_sequence)).collect::<HashMap<_, _>>();
	let object_textures = transform_object_textures::<Od, Oc>(object_textures);
	let entities = entities.iter().into_group_map_by(|e| e.room_index as usize);
	let mut textured_vertices = vec![];
	let mut solid_vertices = vec![];
	let mut sprite_vertices = vec![];
	let room_vertex_indices = rooms.iter().enumerate().map(|(room_index, room)| {
		let solid_start = solid_vertices.len();
		let sprite_start = sprite_vertices.len();
		let room_verts = room
			.vertices
			.iter()
			.map(|tr::RoomVertex { vertex, .. }| vertex.as_ivec3())
			.map(|IVec3 { x, y, z }| ivec3(x + room.x, y, z + room.z).as_vec3() * WORLD_COORD_SCALE)
			.collect::<Vec<_>>();
		let mut opaque = vec![];
		let mut additive = vec![];
		add_textured_faces(&mut opaque, &mut additive, &object_textures, &room_verts, &room.tris);
		add_textured_faces(&mut opaque, &mut additive, &object_textures, &room_verts, &room.quads);
		for room_static_mesh in room.room_static_meshes.iter() {
			let transform = Mat4::from_translation(room_static_mesh.pos.as_vec3() * WORLD_COORD_SCALE) * Mat4::from_rotation_y(room_static_mesh.rotation as f32 / 65536.0 * TAU);
			T::add_mesh(
				&mut opaque,
				&mut additive,
				&mut solid_vertices,
				&object_textures,
				transform,
				meshes.get_mesh(static_meshes[&room_static_mesh.static_mesh_id].mesh_id),
			);
		}
		if let Some(entities) = entities.get(&room_index) {
			for &entity in entities {
				match models.get(&entity.model_id) {
					Some(model) => {
						let frame = frame_data.get_frame::<T>(model.frame_byte_offset, model.num_meshes);
						let entity_transform = Mat4::from_translation(entity.pos.as_vec3() * WORLD_COORD_SCALE) * Mat4::from_rotation_y(entity.rotation as f32 / 65536.0 * TAU);
						let transform = Mat4::from_translation(frame.offset.as_vec3() * WORLD_COORD_SCALE) * get_rotation::<T>(frame.rotations[0]);
						T::add_mesh(
							&mut opaque,
							&mut additive,
							&mut solid_vertices,
							&object_textures,
							entity_transform * transform,
							meshes.get_mesh(model.mesh_id),
						);
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
							let transform = parent * Mat4::from_translation(mesh_node.offset.as_vec3() * WORLD_COORD_SCALE) * get_rotation::<T>(frame.rotations[index + 1]);
							T::add_mesh(
								&mut opaque,
								&mut additive,
								&mut solid_vertices,
								&object_textures,
								entity_transform * transform,
								meshes.get_mesh(model.mesh_id + index as u16 + 1),
							);
							last_transform = transform;
						}
					},
					None => match sprite_sequences.get(&entity.model_id) {
						Some(&sprite_sequence) => {
							let spr_tex = &sprite_textures[sprite_sequence.sprite_index as usize];
							for &i in QUAD_INDICES.single {
								sprite_vertices.push(SpriteVertex {
									pos: entity.pos.as_vec3() * WORLD_COORD_SCALE,
									uv: spr_tex[i].uv,
									offset2d: spr_tex[i].world_offset,
								});
							}
						},
						None => println!("sprite sequence id {} not found", entity.model_id),
					},
				}
			}
		}
		let opaque_start = textured_vertices.len();
		let opaque_end = opaque_start + opaque.len();
		let additive_start = opaque_end;
		let additive_end = additive_start + additive.len();
		let solid_end = solid_vertices.len();
		let sprite_end = sprite_vertices.len();
		textured_vertices.extend(opaque);
		textured_vertices.extend(additive);
		RoomVertexIndices {
			opaque: opaque_start as u32..opaque_end as u32,
			additive: additive_start as u32..additive_end as u32,
			solid: solid_start as u32..solid_end as u32,
			sprite: sprite_start as u32..sprite_end as u32,
		}
	}).collect();
	let mut static_room_indices = (0..rooms.len()).collect::<Vec<_>>();
	let mut flip_groups = HashMap::<u8, Vec<FlipRoom>>::new();
	for (index, room) in rooms.iter().enumerate() {
		if let Some(flip_index) = room.flip_room_index {
			let flip_index = flip_index.get() as usize;
			static_room_indices.remove(static_room_indices.binary_search(&index).expect("no_flips missing index"));
			static_room_indices.remove(static_room_indices.binary_search(&flip_index).expect("no_flips missing flip index"));
			flip_groups.entry(T::flip_group(&room.extra)).or_default().push(FlipRoom { original: index, flipped: flip_index });
		}
	}
	let mut flip_groups = flip_groups
		.into_iter()
		.map(|(label, flip_rooms)| FlipGroup { label, flip_rooms, flipped: false })
		.collect::<Vec<_>>();
	flip_groups.sort_by_key(|flip_group| flip_group.label);
	LevelRenderData {
		atlas_size,
		atlas_data,
		textured_vertices,
		solid: palette.map(|palette| SolidData { palette, solid_vertices }),
		sprite_vertices,
		room_vertex_indices,
		static_room_indices,
		flip_groups,
	}
}

pub fn load_level_render_data(path: &str) -> Result<LevelRenderData> {
	let mut reader = BufReader::new(File::open(path)?);
	let version = reader.read_u32::<LE>()?;
	reader.rewind()?;
	match version {
		32 => todo!("tr1"),
		45 => tr2::load_level_render_data(&mut reader),
		4278714424 | 4279763000 => tr3::load_level_render_data(&mut reader),
		3428948 => match path.rfind('.') {
			Some(last_period) => match path[last_period + 1..].to_lowercase().as_str() {
				"tr4" => tr4::load_level_render_data(&mut reader),
				"trc" => todo!("tr5"),
				a => panic!("unknown file extension: {}", a),
			},
			None => panic!("unable to determine file type, rename with appropriate extension"),
		},
		a => panic!("unsupported version: {}", a),
	}
}
