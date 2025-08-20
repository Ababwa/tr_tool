use std::collections::HashMap;
use tr_model::tr1;
use crate::{boxed_slice::{self, Bsf}, tr_traits::{Entity, Level, Model, Room}};
use super::{FlipState, RoomRenderData};

pub fn get_static_mesh_map(static_meshes: &[tr1::StaticMesh]) -> HashMap<u16, &tr1::StaticMesh> {
	let mut map = HashMap::with_capacity(static_meshes.len());
	for static_mesh in static_meshes {
		map.insert(static_mesh.id as u16, static_mesh);
	}
	map
}

enum ModelRef<'a, M> {
	Model(&'a M),
	SpriteSequence(&'a tr1::SpriteSequence),
}

//TODO: check ids dont overlap
fn get_model_map<'a, M: Model>(
	models: &'a [M],
	sprite_sequences: &'a [tr1::SpriteSequence],
) -> HashMap<u16, ModelRef<'a, M>> {
	let size = models.len() + sprite_sequences.len();
	let mut map = HashMap::with_capacity(size);
	for model in models {
		map.insert(model.id() as u16, ModelRef::Model(model));
	}
	for ss in sprite_sequences {
		map.insert(ss.id as u16, ModelRef::SpriteSequence(ss));
	}
	map
}

pub struct ModelEntity<'a, M> {
	pub entity_index: usize,
	pub model: &'a M,
}

pub struct SpriteEntity<'a> {
	pub entity_index: usize,
	pub sprite_sequence: &'a tr1::SpriteSequence,
}

pub struct RoomEntities<'a, M> {
	pub model_entities: Vec<ModelEntity<'a, M>>,
	pub sprite_entities: Vec<SpriteEntity<'a>>,
}

fn get_entities_by_room_inner<'a, E: Entity, M: Model>(
	entities: &'a [E],
	num_rooms: usize,
	model_map: &HashMap<u16, ModelRef<'a, M>>,
) -> Vec<RoomEntities<'a, M>> {
	//count
	let mut room_entity_counts = vec![[0, 0]; num_rooms];
	for entity in entities {
		let kind = match model_map[&entity.model_id()] {
			ModelRef::Model(_) => 0,
			ModelRef::SpriteSequence(_) => 1,
		};
		room_entity_counts[entity.room_index() as usize][kind] += 1;
	}
	//allocate
	let mut entities_by_room = Vec::with_capacity(num_rooms);
	for [model_entities, sprite_entities] in room_entity_counts {
		let room_entities = RoomEntities {
			model_entities: Vec::with_capacity(model_entities),
			sprite_entities: Vec::with_capacity(sprite_entities),
		};
		entities_by_room.push(room_entities);
	}
	//fill
	for entity_index in 0..entities.len() {
		let entity = &entities[entity_index];
		let room_entities = &mut entities_by_room[entity.room_index() as usize];
		match model_map[&entity.model_id()] {
			ModelRef::Model(model) => {
				let model_entity = ModelEntity {
					entity_index,
					model,
				};
				room_entities.model_entities.push(model_entity);
			},
			ModelRef::SpriteSequence(sprite_sequence) => {
				let sprite_entity = SpriteEntity {
					entity_index,
					sprite_sequence,
				};
				room_entities.sprite_entities.push(sprite_entity);
			},
		}
	}
	entities_by_room
}

pub fn get_entities_by_room<L: Level>(level: &L) -> Vec<RoomEntities<L::Model>> {
	let model_map = get_model_map(level.models(), level.sprite_sequences());
	get_entities_by_room_inner(level.entities(), level.rooms().len(), &model_map)
}

pub struct FlipGroup {
	pub number: u8,
	pub flipped: bool,
	/// First half are original indices, second half are flipped indices.
	pub room_indices: Box<[usize]>,
	/// Count of preceding flip rooms. Offset into `LoadedLevel::all_room_indices`.
	pub offset: usize,
}

impl FlipGroup {
	pub fn active_indices(&self) -> &[usize] {
		let half = self.room_indices.len() / 2;
		&self.room_indices[self.flipped as usize * half..][..half]
	}
}

pub fn get_flip_groups<R: Room>(
	rooms: &[R],
	room_render_data: &mut [RoomRenderData],
) -> (Box<[usize]>, Box<[FlipGroup]>) {
	#[derive(Clone, Copy)]
	enum RoomState {
		Static,
		FlipOriginal,
		FlipFlipped,
	}
	let mut room_states = boxed_slice::new_copied(RoomState::Static, rooms.len());
	let mut num_static_rooms = rooms.len();
	let mut num_flip_groups = 0;
	let mut flip_group_counts = [0u8; 256];
	for room_index in 0..rooms.len() {
		let room = &rooms[room_index];
		let flip_index = room.flip_room_index();
		if
			flip_index != u16::MAX &&
			matches!(room_states[room_index], RoomState::Static) &&
			matches!(room_states[flip_index as usize], RoomState::Static)
		{
			room_states[room_index] = RoomState::FlipOriginal;
			room_states[flip_index as usize] = RoomState::FlipFlipped;
			let group = room.flip_group();
			let orig_state = FlipState {
				group,
				original: true,
				other_index: flip_index,
			};
			let flip_state = FlipState {
				group,
				original: false,
				other_index: room_index as u16,
			};
			room_render_data[room_index].flip_state = Some(orig_state);
			room_render_data[flip_index as usize].flip_state = Some(flip_state);
			let flip_group_count = &mut flip_group_counts[group as usize];
			if *flip_group_count == 0 {
				num_flip_groups += 1;
			}
			*flip_group_count += 1;
			num_static_rooms -= 2;
		}
	}
	let mut flip_groups = Bsf::new(num_flip_groups);
	let mut offset = 0;
	for fg_num in 0..256 {
		let count = flip_group_counts[fg_num];
		if count > 0 {
			let room_indices = boxed_slice::new_copied(usize::MAX, count as usize * 2);//init out of order
			let flip_group = FlipGroup {
				number: fg_num as u8,
				flipped: false,
				room_indices,
				offset,
			};
			flip_groups.push(flip_group);
			offset += count as usize;
		}
	}
	let mut flip_groups = flip_groups.into_boxed_slice();
	let mut static_room_indices = Bsf::new(num_static_rooms);
	for room_index in 0..rooms.len() {
		match room_states[room_index] {
			RoomState::Static => static_room_indices.push(room_index),
			RoomState::FlipOriginal => {
				let room = &rooms[room_index];
				let flip_room_index = room.flip_room_index() as usize;
				let fg_num = room.flip_group();
				let fg_index = flip_groups.binary_search_by_key(&fg_num, |f| f.number).unwrap();
				let room_indices = &mut flip_groups[fg_index].room_indices;
				let fg_rooms = room_indices.len() / 2;
				let fg_left = &mut flip_group_counts[fg_num as usize];
				room_indices[fg_rooms - *fg_left as usize] = room_index;
				room_indices[2 * fg_rooms - *fg_left as usize] = flip_room_index;
				*fg_left -= 1;
			},
			RoomState::FlipFlipped => {},
		}
	}
	(static_room_indices.into_boxed_slice(), flip_groups)
}
