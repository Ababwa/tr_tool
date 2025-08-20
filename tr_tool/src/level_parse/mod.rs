mod counts;
mod geom_writer;
mod instance_writer;
mod maps;

use std::{collections::{hash_map::Entry, HashMap}, f32::consts::TAU, io::{self, BufRead, Seek}, ops::Range};
use glam::{IVec3, Mat4, Vec3};
use tr_model::{tr1, tr2, tr3, tr4, tr5};
use wgpu::{BindGroup, BindingResource, Buffer, BufferUsages, Device, Queue, TextureFormat};
use crate::{
	as_bytes::AsBytes, boxed_slice::Bsf, gfx::{self, bind_group_entry as entry},
	object_data::{self, MeshFaceData}, render_resources::{RenderResources, ATLASES_ENTRY, PALETTE_ENTRY},
	tr_traits::{
		Entity, Frame, Layer, Level, LevelStore, Mesh, Model, ObjectTexture, Room, RoomStaticMesh,
		RoomVertex, RoomVertexPos, Version,
	},
};
use counts::Counts;
use geom_writer::{FaceArrayIndex, GeomOutput, GeomWriter, TransformIndex};
use instance_writer::{Instances, InstanceWriter, RoomFaceOffsets, TexturedMeshFaceOffsets};
use maps::{FlipGroup, ModelEntity, RoomEntities};

pub struct LayerOffsets {
	pub quads: RoomFaceOffsets,
	pub tris: RoomFaceOffsets,
}

#[derive(Default)]
pub struct MeshOffsets {
	pub textured_quads: TexturedMeshFaceOffsets,
	pub textured_tris: TexturedMeshFaceOffsets,
	pub solid_quads: Range<u32>,
	pub solid_tris: Range<u32>,
}

pub struct FlipState {
	pub group: u8,
	pub original: bool,
	pub other_index: u16,
}

//TODO: Collapse layers, static meshes, and entity meshes.
pub struct RoomRenderData {
	pub layers: Box<[LayerOffsets]>,
	pub static_meshes: Box<[MeshOffsets]>,
	pub entity_meshes: Box<[Box<[MeshOffsets]>]>,
	pub room_sprites: Range<u32>,
	pub entity_sprites: Range<u32>,
	pub center: Vec3,
	pub radius: f32,
	pub flip_state: Option<FlipState>,
}

pub struct BindGroups {
	pub palette_bg: Option<BindGroup>,
	pub texture_16bit_bg: Option<BindGroup>,
	pub texture_32bit_bg: Option<BindGroup>,
	pub solid_32bit_bg: Option<BindGroup>,
	pub misc_images_bg: Option<(BindGroup, u32)>,
}

/// All that must be extracted from the level file via the `tr_traits::Level` interface.
pub struct LevelData {
	pub geom: GeomOutput,
	pub object_data: Box<[object_data::ObjectData]>,
	pub face_instance_buffer: Buffer,
	pub sprite_instance_buffer: Buffer,
	pub room_render_data: Box<[RoomRenderData]>,
	/// Indices of non-flip rooms.
	pub static_room_indices: Box<[usize]>,
	/// Indices of flip rooms. Each group may have its "original" indices or "flipped" indices active.
	pub flip_groups: Box<[FlipGroup]>,
	pub bind_groups: BindGroups,
	pub num_atlases: u32,
	pub object_texture_size: u32,
	pub level: LevelStore,
}

struct WrittenMesh {
	textured_quads: FaceArrayIndex,
	textured_tris: FaceArrayIndex,
	solid_quads: FaceArrayIndex,
	solid_tris: FaceArrayIndex,
}

fn make_transform(angle: u16, pos: IVec3) -> Mat4 {
	Mat4::from_translation(pos.as_vec3()) * Mat4::from_rotation_y(angle as f32 / 65536.0 * TAU)
}

fn get_model_transforms<L: Level>(level: &L, model: &L::Model) -> Vec<Mat4> {
	let frame = level.get_frame(model);
	let mut rotations = frame.iter_rotations();
	let first_rotation = rotations.next().expect("model has no rotations");
	let first_transform = Mat4::from_translation(frame.offset().as_vec3()) * first_rotation;
	let mut transforms = Vec::with_capacity(model.num_meshes() as usize);
	let mut last_transform = 0;
	transforms.push(first_transform);
	let mesh_nodes = level.get_mesh_nodes(model);
	let mut parent_stack = Vec::with_capacity(mesh_nodes.len());
	for mesh_node in mesh_nodes {
		let rotation = rotations.next().expect("model has insufficient rotations");
		let delta = Mat4::from_translation(mesh_node.offset.as_vec3()) * rotation;
		let parent = match (mesh_node.flags.pop(), mesh_node.flags.push()) {
			(true, true) => *parent_stack.last().expect("transform stack empty on peek"),
			(true, false) => parent_stack.pop().expect("transform stack empty on pop"),
			(false, true) => {
				parent_stack.push(last_transform);
				last_transform
			},
			(false, false) => last_transform
		};
		let transform = transforms[parent] * delta;
		last_transform = transforms.len();
		transforms.push(transform);
	}
	transforms
}

fn min_max<V: RoomVertex>(room_vertices: &[V]) -> Option<(Vec3, Vec3)> {
	let (first, rest) = room_vertices.split_first()?;
	let mut min = first.pos().as_vec3();
	let mut max = min;
	for vertex in rest {
		let pos = vertex.pos().as_vec3();
		min = min.min(pos);
		max = max.max(pos);
	}
	Some((min, max))
}

fn write_mesh<M: Mesh>(geom_writer: &mut GeomWriter, mesh: &M) -> WrittenMesh {
	let vertex_array = geom_writer.vertex_array(mesh.vertices());
	let textured_quads = geom_writer.face_array(mesh.textured_quads(), vertex_array);
	let textured_tris = geom_writer.face_array(mesh.textured_tris(), vertex_array);
	let solid_quads = geom_writer.face_array(mesh.solid_quads(), vertex_array);
	let solid_tris = geom_writer.face_array(mesh.solid_tris(), vertex_array);
	WrittenMesh {
		textured_quads,
		textured_tris,
		solid_quads,
		solid_tris,
	}
}

fn write_mesh_instance<M: Mesh, O: ObjectTexture, D: MeshFaceData>(
	instance_writer: &mut InstanceWriter,
	object_textures: &[O],
	transform_index: TransformIndex,
	face_data: D,
	written_mesh: &WrittenMesh,
	mesh: &M,
) -> MeshOffsets {
	let tq_data = face_data.with_face_kind(object_data::MeshFaceKind::TexturedQuad);
	let textured_quads = instance_writer.textured_mesh_face_array(
		object_textures,
		transform_index,
		written_mesh.textured_quads,
		tq_data,
		mesh.textured_quads(),
	);
	let tt_data = face_data.with_face_kind(object_data::MeshFaceKind::TexturedTri);
	let textured_tris = instance_writer.textured_mesh_face_array(
		object_textures,
		transform_index,
		written_mesh.textured_tris,
		tt_data,
		mesh.textured_tris(),
	);
	let sq_data = face_data.with_face_kind(object_data::MeshFaceKind::SolidQuad);
	let solid_quads = instance_writer.solid_face_array(
		transform_index,
		written_mesh.solid_quads,
		sq_data,
		mesh.solid_quads(),
	);
	let st_data = face_data.with_face_kind(object_data::MeshFaceKind::SolidTri);
	let solid_tris = instance_writer.solid_face_array(
		transform_index,
		written_mesh.solid_tris,
		st_data,
		mesh.solid_tris(),
	);
	MeshOffsets {
		textured_quads,
		textured_tris,
		solid_quads,
		solid_tris,
	}
}

fn write_layer<O: ObjectTexture, R: Room>(
	geom_writer: &mut GeomWriter,
	instance_writer: &mut InstanceWriter,
	object_textures: &[O],
	room_index: u16,
	transform_index: TransformIndex,
	layer: Layer<R>,
) -> LayerOffsets {
	let vertex_array_offset = geom_writer.vertex_array(layer.vertices);
	let quads_offset = geom_writer.face_array(layer.quads, vertex_array_offset);
	let tris_offset = geom_writer.face_array(layer.tris, vertex_array_offset);
	let quad_data = object_data::RoomFace {
		room_index,
		layer_index: layer.index as u8,
		face_kind: object_data::RoomFaceKind::Quad,
		face_index: 0,
	};
	let quads = instance_writer.room_face_array(
		object_textures,
		transform_index,
		quads_offset,
		quad_data,
		layer.quads,
	);
	let tri_data = object_data::RoomFace {
		face_kind: object_data::RoomFaceKind::Tri,
		..quad_data
	};
	let tris = instance_writer.room_face_array(
		object_textures,
		transform_index,
		tris_offset,
		tri_data,
		layer.tris,
	);
	LayerOffsets {
		quads,
		tris,
	}
}

fn write_static_mesh<L: Level, R: RoomStaticMesh>(
	geom_writer: &mut GeomWriter,
	instance_writer: &mut InstanceWriter,
	written_meshes: &mut HashMap<u32, WrittenMesh>,
	level: &L,
	static_mesh_map: &HashMap<u16, &tr1::StaticMesh>,
	room_index: u16,
	room_static_mesh_index: u8,
	room_static_mesh: &R,
) -> MeshOffsets {
	let Some(static_mesh) = static_mesh_map.get(&room_static_mesh.static_mesh_id()) else {
		return MeshOffsets::default();
	};
	let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
	let mesh = level.get_mesh(mesh_offset);
	let written_mesh: &_ = match written_meshes.entry(mesh_offset) {
		Entry::Occupied(entry) => entry.into_mut(),
		Entry::Vacant(entry) => {
			let written_mesh = write_mesh(geom_writer, &mesh);
			entry.insert(written_mesh)
		},
	};
	let transform = make_transform(room_static_mesh.angle(), room_static_mesh.pos());
	let transform_index = geom_writer.transform(&transform);
	let face_data = object_data::StaticMeshFace {
		room_index,
		room_static_mesh_index,
		face_kind: object_data::MeshFaceKind::TexturedQuad,
		face_index: 0,
	};
	write_mesh_instance(
		instance_writer,
		level.object_textures(),
		transform_index,
		face_data,
		written_mesh,
		&mesh,
	)
}

fn write_entity_mesh<L: Level>(
	geom_writer: &mut GeomWriter,
	instance_writer: &mut InstanceWriter,
	written_meshes: &mut HashMap<u32, WrittenMesh>,
	level: &L,
	entity_index: u16,
	mesh_index: u8,
	transform: &Mat4,
	mesh_offset_index: usize,
) -> MeshOffsets {
	let mesh_offset = level.mesh_offsets()[mesh_offset_index];
	let mesh = level.get_mesh(mesh_offset);
	let written_mesh: &_ = match written_meshes.entry(mesh_offset) {
		Entry::Occupied(entry) => entry.into_mut(),
		Entry::Vacant(entry) => {
			let written_mesh = write_mesh(geom_writer, &mesh);
			entry.insert(written_mesh)
		},
	};
	let transform_index = geom_writer.transform(transform);
	let face_data = object_data::EntityMeshFace {
		entity_index,
		mesh_index,
		face_kind: object_data::MeshFaceKind::TexturedQuad,
		face_index: 0,
	};
	write_mesh_instance(
		instance_writer,
		level.object_textures(),
		transform_index,
		face_data,
		written_mesh,
		&mesh,
	)
}

fn write_model_entity<L: Level>(
	geom_writer: &mut GeomWriter,
	instance_writer: &mut InstanceWriter,
	written_meshes: &mut HashMap<u32, WrittenMesh>,
	processed_models: &mut HashMap<u16, Vec<Mat4>>,
	level: &L,
	entity_index: u16,
	entity: &L::Entity,
	model: &L::Model,
) -> Box<[MeshOffsets]> {
	let entity_transform = make_transform(entity.angle(), entity.pos());
	let transforms: &[_] = match processed_models.entry(entity.model_id()) {
		Entry::Occupied(entry) => entry.into_mut(),
		Entry::Vacant(entry) => {
			let transforms = get_model_transforms(level, model);
			entry.insert(transforms)
		},
	};
	let mut entity_meshes = Bsf::new(model.num_meshes() as usize);
	let mesh_offset_start = model.mesh_offset_index() as usize;
	for mesh_index in 0..model.num_meshes() as usize {
		let transform = entity_transform * transforms[mesh_index];
		let mesh_offset_index = mesh_offset_start + mesh_index;
		let mesh_offsets = write_entity_mesh(
			geom_writer,
			instance_writer,
			written_meshes,
			level,
			entity_index,
			mesh_index as u8,
			&transform,
			mesh_offset_index,
		);
		entity_meshes.push(mesh_offsets);
	}
	entity_meshes.into_boxed_slice()
}

fn write_room<L: Level>(
	geom_writer: &mut GeomWriter,
	instance_writer: &mut InstanceWriter,
	written_meshes: &mut HashMap<u32, WrittenMesh>,
	processed_models: &mut HashMap<u16, Vec<Mat4>>,
	level: &L,
	static_mesh_map: &HashMap<u16, &tr1::StaticMesh>,
	room_index: u16,
	room_entities: &RoomEntities<L::Model>,
	room: &L::Room,
) -> RoomRenderData {
	let room_pos = room.pos().as_vec3();
	let transform = Mat4::from_translation(room_pos);
	let transform_index = geom_writer.transform(&transform);
	let mut layers = Bsf::new(room.num_layers());
	for layer in room.iter_layers() {
		let layer_offsets = write_layer(
			geom_writer,
			instance_writer,
			level.object_textures(),
			room_index,
			transform_index,
			layer,
		);
		layers.push(layer_offsets);
	}
	let mut static_meshes = Bsf::new(room.room_static_meshes().len());
	for room_static_mesh_index in 0..room.room_static_meshes().len() {
		let room_static_mesh = &room.room_static_meshes()[room_static_mesh_index];
		let static_mesh_offsets = write_static_mesh(
			geom_writer,
			instance_writer,
			written_meshes,
			level,
			static_mesh_map,
			room_index,
			room_static_mesh_index as u8,
			room_static_mesh,
		);
		static_meshes.push(static_mesh_offsets);
	}
	let mut entity_meshes = Bsf::new(room_entities.model_entities.len());
	for &ModelEntity { entity_index, model } in &room_entities.model_entities {
		let entity = &level.entities()[entity_index];
		let entity_mesh_offsets = write_model_entity(
			geom_writer,
			instance_writer,
			written_meshes,
			processed_models,
			level,
			entity_index as u16,
			entity,
			model,
		);
		entity_meshes.push(entity_mesh_offsets);
	}
	let room_sprites = instance_writer.room_sprites(
		room.vertices(),
		room.pos(),
		room_index,
		room.sprites(),
	);
	let entity_sprites = instance_writer.entity_sprites(level.entities(), &room_entities.sprite_entities);
	let (center, radius) = match min_max(room.vertices()) {
		Some((min, max)) => {
			let center = 0.5 * (min + max);
			let radius = (max - min).max_element();
			(center, radius)
		},
		None => (Vec3::ZERO, 0.0),
	};
	let center = center + room_pos;
	RoomRenderData {
		layers: layers.into_boxed_slice(),
		static_meshes: static_meshes.into_boxed_slice(),
		entity_meshes: entity_meshes.into_boxed_slice(),
		room_sprites,
		entity_sprites,
		center,
		radius,
		flip_state: None,
	}
}

fn bind_groups<L: Level>(device: &Device, queue: &Queue, rr: &RenderResources, level: &L) -> BindGroups {
	let mut palette_bg = None;
	let mut texture_16bit_bg = None;
	let mut texture_32bit_bg = None;
	let mut solid_32bit_bg = None;
	let mut misc_images_bg = None;
	let mut common_entries = rr.binding_buffers.entries();
	let bgls = &rr.bind_group_layouts;
	if let (Some(atlases), Some(palette)) = (level.atlases_palette(), level.palette_24bit()) {
		let palette_view = gfx::palette_view(device, queue, palette);
		let atlases_view = gfx::atlases_view(device, queue, atlases, TextureFormat::R8Uint);
		let palette_entry = entry(PALETTE_ENTRY, BindingResource::TextureView(&palette_view));
		let atlases_entry = entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = common_entries.with_both(palette_entry, atlases_entry);
		let bind_group = gfx::bind_group(device, &bgls.palette_bgl, entries);
		palette_bg = Some(bind_group);
	}
	if let Some(palette) = level.palette_32bit() {
		let palette_view = gfx::palette_view(device, queue, palette);
		let palette_entry = entry(PALETTE_ENTRY, BindingResource::TextureView(&palette_view));
		let entries = common_entries.with(palette_entry);
		let bind_group = gfx::bind_group(device, &bgls.solid_32bit_bgl, entries);
		solid_32bit_bg = Some(bind_group);
	}
	if let Some(atlases) = level.atlases_16bit() {
		let atlases_view = gfx::atlases_view(device, queue, atlases, TextureFormat::R16Uint);
		let atlases_entry = entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = common_entries.with(atlases_entry);
		let bind_group = gfx::bind_group(device, &bgls.texture_bgl, entries);
		texture_16bit_bg = Some(bind_group);
	}
	if let Some(atlases) = level.atlases_32bit() {
		let atlases_view = gfx::atlases_view(device, queue, atlases, TextureFormat::R32Uint);
		let atlases_entry = entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = common_entries.with(atlases_entry);
		let bind_group = gfx::bind_group(device, &bgls.texture_bgl, entries);
		texture_32bit_bg = Some(bind_group);
	}
	if let Some(misc_images) = level.misc_images() {
		let atlases_view = gfx::atlases_view(device, queue, misc_images, TextureFormat::R32Uint);
		let atlases_entry = entry(ATLASES_ENTRY, BindingResource::TextureView(&atlases_view));
		let entries = common_entries.with(atlases_entry);
		let bind_group = gfx::bind_group(device, &bgls.texture_bgl, entries);
		misc_images_bg = Some((bind_group, misc_images.len() as u32));
	}
	BindGroups {
		palette_bg,
		texture_16bit_bg,
		texture_32bit_bg,
		solid_32bit_bg,
		misc_images_bg,
	}
}

fn parse_level<L: Level>(device: &Device, queue: &Queue, rr: &RenderResources, level: L) -> LevelData {
	let bind_groups = bind_groups(device, queue, rr, &level);
	let static_mesh_map = maps::get_static_mesh_map(level.static_meshes());
	let entities_by_room = maps::get_entities_by_room(&level);
	let counts = Counts::new(&level, &static_mesh_map, &entities_by_room);
	let mut geom_writer = GeomWriter::new(level.object_textures(), level.sprite_textures(), &counts);
	let mut instance_writer = InstanceWriter::new(&counts);
	let mut written_meshes = HashMap::with_capacity(counts.meshes);//TODO: try preprocess meshes and models
	let mut processed_models = HashMap::with_capacity(level.models().len());
	let mut room_render_data = Bsf::new(level.rooms().len());
	for room_index in 0..level.rooms().len() {
		let room = &level.rooms()[room_index];
		let room_entities = &entities_by_room[room_index];
		let rrd = write_room(
			&mut geom_writer,
			&mut instance_writer,
			&mut written_meshes,
			&mut processed_models,
			&level,
			&static_mesh_map,
			room_index as u16,
			room_entities,
			room,
		);
		room_render_data.push(rrd);
	}
	let mut room_render_data = room_render_data.into_boxed_slice();
	let (static_room_indices, flip_groups) = maps::get_flip_groups(level.rooms(), &mut room_render_data);
	let Instances { face_instances, sprite_instances, object_data } = instance_writer.done();
	let face_instance_buffer = gfx::buffer_init(
		device,
		face_instances[..].as_bytes(),
		BufferUsages::VERTEX,
	);
	let sprite_instance_buffer = gfx::buffer_init(
		device,
		sprite_instances[..].as_bytes(),
		BufferUsages::VERTEX,
	);
	LevelData {
		geom: geom_writer.done(),
		face_instance_buffer,
		sprite_instance_buffer,
		object_data,
		room_render_data,
		static_room_indices,
		flip_groups,
		bind_groups,
		num_atlases: level.num_atlases() as u32,
		object_texture_size: (size_of::<L::ObjectTexture>() / 2) as u32,
		level: level.store(),
	}
}

fn read_level<R: BufRead + Seek, L: Level>(
	device: &Device,
	queue: &Queue,
	rr: &RenderResources,
	reader: &mut R,
) -> io::Result<LevelData> {
	let level = L::read(reader)?;
	let level_data = parse_level(device, queue, rr, level);
	Ok(level_data)
}

impl LevelData {
	pub fn load<R: BufRead + Seek>(
		device: &Device,
		queue: &Queue,
		rr: &RenderResources,
		reader: &mut R,
		version: Version,
	) -> io::Result<Self> {
		match version {
			Version::Tr1 => read_level::<R, tr1::Level>(device, queue, rr, reader),
			Version::Tr2 => read_level::<R, tr2::Level>(device, queue, rr, reader),
			Version::Tr3 => read_level::<R, tr3::Level>(device, queue, rr, reader),
			Version::Tr4 => read_level::<R, tr4::Level>(device, queue, rr, reader),
			Version::Tr5 => read_level::<R, tr5::Level>(device, queue, rr, reader),
		}
	}
}
