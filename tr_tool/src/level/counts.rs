use std::{collections::{hash_map::Entry, HashMap}};
use tr_model::tr1;
use crate::{round_up, tr_traits::{Level, Mesh, Model, Room, RoomStaticMesh}};
use super::{geom_writer::{FACE_ARRAY_HEADER_SIZE, VERTEX_ARRAY_HEADER_SIZE}, maps::RoomEntities};

/// Assert assumptions about level structure. Failure indicates a need to change assumptions.
macro_rules! size_assert {
	($len:expr, u8) => { assert!($len <= 256) };
	($len:expr, u16) => { assert!($len <= 65536) };
}

#[derive(Default)]
pub struct Counts {
	pub vertex_arrays_size: usize,
	pub vertex_arrays: usize,
	pub face_arrays_size: usize,
	pub face_arrays: usize,
	pub meshes: usize,
	pub transforms: usize,
	pub face_instances: usize,
	pub sprite_instances: usize,
	pub model_entities: usize,
}

impl Counts {
	fn vertex_array<V>(&mut self, vertices: &[V]) {
		self.vertex_arrays_size += VERTEX_ARRAY_HEADER_SIZE + round_up::<4>(size_of_val(vertices));
		self.vertex_arrays += 1;
	}
	
	fn face_array<F>(&mut self, faces: &[F]) {
		self.face_arrays_size += FACE_ARRAY_HEADER_SIZE + round_up::<4>(size_of_val(faces));
		self.face_arrays += 1;
	}
	
	fn mesh<'a, M: Mesh<'a> + 'a>(&mut self, mesh: &M) {
		self.vertex_array(mesh.vertices());
		self.face_array(mesh.textured_quads());
		self.face_array(mesh.textured_tris());
		self.face_array(mesh.solid_quads());
		self.face_array(mesh.solid_tris());
		self.meshes += 1;
	}
	
	fn mesh_instance<'a, M: Mesh<'a> + 'a>(&mut self, mesh: &M) {
		self.transforms += 1;
		self.face_instances +=
			mesh.textured_quads().len() +
			mesh.textured_tris().len() +
			mesh.solid_quads().len() +
			mesh.solid_tris().len();
	}
	
	fn static_mesh<L: Level, R: RoomStaticMesh>(
		&mut self,
		level: &L,
		static_mesh_map: &HashMap<u16, &tr1::StaticMesh>,
		counted_meshes: &mut HashMap<u32, ()>,
		room_static_mesh: &R,
	) {
		let static_mesh = static_mesh_map[&room_static_mesh.static_mesh_id()];
		let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
		let mesh = level.get_mesh(mesh_offset);
		if let Entry::Vacant(entry) = counted_meshes.entry(mesh_offset) {
			entry.insert(());
			self.mesh(&mesh);
		}
		self.mesh_instance(&mesh);
	}
	
	fn model_entity<L: Level>(
		&mut self,
		level: &L,
		counted_meshes: &mut HashMap<u32, ()>,
		model: &L::Model,
	) {
		size_assert!(model.num_meshes(), u8);
		let mesh_offset_start = model.mesh_offset_index() as usize;
		let mesh_offset_end = mesh_offset_start + model.num_meshes() as usize;
		for mesh_offset_index in mesh_offset_start..mesh_offset_end {
			let mesh_offset = level.mesh_offsets()[mesh_offset_index];
			let mesh = level.get_mesh(mesh_offset);
			self.mesh_instance(&mesh);
			if let Entry::Vacant(entry) = counted_meshes.entry(mesh_offset) {
				entry.insert(());
				self.mesh(&mesh);
			}
		}
	}
	
	fn room<L: Level>(
		&mut self,
		level: &L,
		static_mesh_map: &HashMap<u16, &tr1::StaticMesh>,
		counted_meshes: &mut HashMap<u32, ()>,
		room_entities: &RoomEntities<L::Model>,
		room: &L::Room,
	) {
		size_assert!(room.num_layers(), u8);
		size_assert!(room.room_static_meshes().len(), u8);
		size_assert!(room.sprites().len(), u16);
		for layer in room.iter_layers() {
			self.vertex_array(layer.vertices);
			self.face_array(layer.quads);
			self.face_array(layer.tris);
			self.face_instances += layer.quads.len() + layer.tris.len();
		}
		for room_static_mesh in room.room_static_meshes() {
			self.static_mesh(level, static_mesh_map, counted_meshes, room_static_mesh);
		}
		for model_entity in &room_entities.model_entities {
			self.model_entity(level, counted_meshes, model_entity.model);
		}
		self.sprite_instances += room.sprites().len() + room_entities.sprite_entities.len();
		self.model_entities += room_entities.model_entities.len();
		self.transforms += 1;
	}
	
	pub fn new<L: Level>(
		level: &L,
		static_mesh_map: &HashMap<u16, &tr1::StaticMesh>,
		entities_by_room: &[RoomEntities<L::Model>],
	) -> Self {
		size_assert!(level.entities().len(), u16);
		let mut counts = Self::default();
		let mut counted_meshes = HashMap::with_capacity(level.mesh_offsets().len());//upper-bound
		for room_index in 0..level.rooms().len() {
			let room = &level.rooms()[room_index];
			let room_entities = &entities_by_room[room_index];
			counts.room(
				level,
				static_mesh_map,
				&mut counted_meshes,
				room_entities,
				room,
			);
		}
		size_assert!(counts.face_arrays, u16);
		size_assert!(counts.transforms, u16);
		size_assert!(counts.sprite_instances, u16);
		counts
	}
}
