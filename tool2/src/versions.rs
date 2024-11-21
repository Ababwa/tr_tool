use std::{f32::consts::TAU, slice::Iter};
use glam::{I16Vec3, IVec3, Mat4, U16Vec3};
use tr_model::{tr1, tr2, tr3};

#[derive(Debug)]
pub enum TrVersion<Tr1, Tr2, Tr3> {
	Tr1(Tr1),
	Tr2(Tr2),
	Tr3(Tr3),
}

macro_rules! converge {
	($val:expr, |$pat:pat_param| $out:expr $(,)?) => {
		match $val {
			TrVersion::Tr1($pat) => $out,
			TrVersion::Tr2($pat) => $out,
			TrVersion::Tr3($pat) => $out,
		}
	};
}
pub(crate) use converge;

macro_rules! parallel {
	($val:expr, |$pat:pat_param| $out:expr) => {
		match $val {
			TrVersion::Tr1($pat) => TrVersion::Tr1($out),
			TrVersion::Tr2($pat) => TrVersion::Tr2($out),
			TrVersion::Tr3($pat) => TrVersion::Tr3($out),
		}
	};
}
pub(crate) use parallel;

pub type Ref<'a, Tr1, Tr2, Tr3> = TrVersion<&'a Tr1, &'a Tr2, &'a Tr3>;

pub type TrIter<'a, Tr1, Tr2, Tr3> = TrVersion<Iter<'a, Tr1>, Iter<'a, Tr2>, Iter<'a, Tr3>>;

impl<'a, Tr1, Tr2, Tr3> Iterator for TrIter<'a, Tr1, Tr2, Tr3> {
	type Item = Ref<'a, Tr1, Tr2, Tr3>;
	
	fn next(&mut self) -> Option<Self::Item> {
		Some(parallel!(self, |iter| iter.next()?))
	}
}

pub type Slice<'a, Tr1, Tr2, Tr3> = Ref<'a, [Tr1], [Tr2], [Tr3]>;

impl<'a, Tr1, Tr2, Tr3> Slice<'a, Tr1, Tr2, Tr3> {
	pub fn len(&self) -> usize {
		converge!(self, |slice| slice.len())
	}
	
	pub fn get(&self, index: usize) -> Ref<'a, Tr1, Tr2, Tr3> {
		parallel!(self, |slice| &slice[index])
	}
	
	pub fn iter(&'a self) -> TrVersion<Iter<'a, Tr1>, Iter<'a, Tr2>, Iter<'a, Tr3>> {
		parallel!(self, |slice| slice.iter())
	}
}

pub type Entity<'a> = Ref<'a, tr1::Entity, tr2::Entity, tr2::Entity>;

impl<'a> Entity<'a> {
	pub fn room_index(&self) -> u16 {
		converge!(self, |entity| entity.room_index)
	}
	
	pub fn model_id(&self) -> u16 {
		converge!(self, |entity| entity.model_id)
	}
	
	pub fn pos(&self) -> IVec3 {
		converge!(self, |entity| entity.pos)
	}
	
	pub fn angle(&self) -> u16 {
		converge!(self, |entity| entity.angle)
	}
}

pub type Mesh<'a> = TrVersion<tr1::Mesh<'a>, tr2::Mesh<'a>, tr2::Mesh<'a>>;

impl<'a> Mesh<'a> {
	pub fn vertices(&self) -> &'a [I16Vec3] {
		converge!(self, |mesh| mesh.vertices)
	}
	
	pub fn textured_quads(&self) -> Slice<'a, tr1::MeshTexturedQuad, tr1::MeshTexturedQuad, tr1::MeshTexturedQuad> {
		parallel!(self, |mesh| mesh.textured_quads)
	}
	
	pub fn textured_tris(&self) -> Slice<'a, tr1::MeshTexturedTri, tr1::MeshTexturedTri, tr1::MeshTexturedTri> {
		parallel!(self, |mesh| mesh.textured_tris)
	}
	
	pub fn solid_quads(&self) -> Slice<'a, tr1::MeshSolidQuad, tr2::MeshSolidQuad, tr2::MeshSolidQuad> {
		parallel!(self, |mesh| mesh.solid_quads)
	}
	
	pub fn solid_tris(&self) -> Slice<'a, tr1::MeshSolidTri, tr2::MeshSolidTri, tr2::MeshSolidTri> {
		parallel!(self, |mesh| mesh.solid_tris)
	}
}

pub type Room<'a> = Ref<'a, tr1::Room, tr2::Room, tr3::Room>;

impl<'a> Room<'a> {
	pub fn get_geom(&self) -> RoomGeom<'a> {
		parallel!(self, |room| room.get_geom())
	}
	
	pub fn pos(&self) -> IVec3 {
		converge!(self, |room| IVec3::new(room.x, 0, room.z))
	}
	
	pub fn room_static_meshes(&self) -> Slice<'a, tr1::RoomStaticMesh, tr2::RoomStaticMesh, tr3::RoomStaticMesh> {
		parallel!(self, |room| &room.room_static_meshes)
	}
	
	pub fn alt_room_index(&self) -> u16 {
		converge!(self, |room| room.alt_room_index)
	}
}

pub type RoomGeom<'a> = TrVersion<tr1::RoomGeom<'a>, tr2::RoomGeom<'a>, tr3::RoomGeom<'a>>;

impl<'a> RoomGeom<'a> {
	pub fn vertices(&self) -> Slice<'a, tr1::RoomVertex, tr2::RoomVertex, tr3::RoomVertex> {
		parallel!(self, |room_geom| room_geom.vertices)
	}
	
	pub fn quads(&self) -> Slice<'a, tr1::RoomQuad, tr1::RoomQuad, tr3::RoomQuad> {
		parallel!(self, |room_geom| room_geom.quads)
	}
	
	pub fn tris(&self) -> Slice<'a, tr1::RoomTri, tr1::RoomTri, tr3::RoomTri> {
		parallel!(self, |room_geom| room_geom.tris)
	}
	
	pub fn sprites(&self) -> &'a [tr1::Sprite] {
		converge!(self, |room_geom| room_geom.sprites)
	}
}

pub type RoomStaticMesh<'a> = Ref<'a, tr1::RoomStaticMesh, tr2::RoomStaticMesh, tr3::RoomStaticMesh>;

impl<'a> RoomStaticMesh<'a> {
	pub fn static_mesh_id(&self) -> u16 {
		converge!(self, |room_static_mesh| room_static_mesh.static_mesh_id)
	}
	
	pub fn pos(&self) -> IVec3 {
		converge!(self, |room_static_mesh| room_static_mesh.pos)
	}
	
	pub fn angle(&self) -> u16 {
		converge!(self, |room_static_mesh| room_static_mesh.angle)
	}
}

pub type RoomVertex<'a> = Ref<'a, tr1::RoomVertex, tr2::RoomVertex, tr3::RoomVertex>;

impl<'a> RoomVertex<'a> {
	pub fn pos(&self) -> I16Vec3 {
		converge!(self, |room_vertex| room_vertex.pos)
	}
}

pub type Frame<'a> = TrVersion<&'a tr1::Frame, tr2::Frame<'a>, tr2::Frame<'a>>;

fn to_radians(angle: u16) -> f32 {
	angle as f32 / 1024.0 * TAU
}

fn to_mat(angles: U16Vec3) -> Mat4 {
	let [x, y, z] = angles.to_array().map(to_radians);
	Mat4::from_rotation_y(y) * Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z)
}

impl<'a> Frame<'a> {
	pub fn offset(&self) -> I16Vec3 {
		match self {
			TrVersion::Tr1(frame) => frame.offset,
			TrVersion::Tr2(frame) | TrVersion::Tr3(frame) => frame.frame_data.offset,
		}
	}
	
	pub fn iter_rotations(&self) -> Box<dyn Iterator<Item = Mat4> + 'a> {
		match self {
			TrVersion::Tr1(frame) => Box::new(frame.rotations.iter().map(|rot| to_mat(rot.get_angles()))),
			TrVersion::Tr2(frame) | TrVersion::Tr3(frame) => Box::new(frame.iter_rotations().map(|rot| {
				match rot {
					tr2::FrameRotation::AllAxes(angles) => to_mat(angles),
					tr2::FrameRotation::SingleAxis(axis, angle) => {
						let angle = to_radians(angle);
						match axis {
							tr2::Axis::X => Mat4::from_rotation_x(angle),
							tr2::Axis::Y => Mat4::from_rotation_y(angle),
							tr2::Axis::Z => Mat4::from_rotation_z(angle),
						}
					},
				}
			})),
		}
	}
}

pub type Level = TrVersion<tr1::Level, tr2::Level, tr3::Level>;

impl Level {
	pub fn static_meshes(&self) -> &[tr1::StaticMesh] {
		converge!(self, |level| &level.static_meshes)
	}
	
	pub fn models(&self) -> &[tr1::Model] {
		converge!(self, |level| &level.models)
	}
	
	pub fn sprite_sequences(&self) -> &[tr1::SpriteSequence] {
		converge!(self, |level| &level.sprite_sequences)
	}
	
	pub fn rooms(&self) -> Slice<tr1::Room, tr2::Room, tr3::Room> {
		parallel!(self, |level| &level.rooms)
	}
	
	pub fn entities(&self) -> Slice<tr1::Entity, tr2::Entity, tr2::Entity> {
		parallel!(self, |level| &level.entities)
	}
	
	pub fn object_textures(&self) -> &[tr1::ObjectTexture] {
		converge!(self, |level| &level.object_textures)
	}
	
	pub fn sprite_textures(&self) -> &[tr1::SpriteTexture] {
		converge!(self, |level| &level.sprite_textures)
	}
	
	pub fn mesh_offsets(&self) -> &[u32] {
		converge!(self, |level| &level.mesh_offsets)
	}
	
	pub fn get_mesh(&self, mesh_offset: u32) -> Mesh {
		parallel!(self, |level| level.get_mesh(mesh_offset))
	}
	
	pub fn get_frame(&self, model: &tr1::Model) -> Frame {
		parallel!(self, |level| level.get_frame(model))
	}
	
	pub fn get_mesh_nodes(&self, model: &tr1::Model) -> &[tr1::MeshNode] {
		converge!(self, |level| level.get_mesh_nodes(model))
	}
	
	pub fn palette_24bit(&self) -> &[tr1::Color24Bit; tr1::PALETTE_LEN] {
		match self {
			TrVersion::Tr1(level) => &level.palette,
			TrVersion::Tr2(level) => &level.palette_24bit,
			TrVersion::Tr3(level) => &level.palette_24bit,
		}
	}
	
	pub fn atlases(&self) -> &[[u8; tr1::ATLAS_PIXELS]] {
		match self {
			TrVersion::Tr1(level) => &level.atlases,
			TrVersion::Tr2(level) => &level.atlases.atlases_palette,
			TrVersion::Tr3(level) => &level.atlases.atlases_palette,
		}
	}
}
