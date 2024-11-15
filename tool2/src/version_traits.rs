use std::f32::consts::TAU;
use glam::{I16Vec3, IVec3, Mat4, U16Vec3};
use tr_model::{
	tr1::{
		self, Color24Bit, MeshNode, MeshTexturedQuad, MeshTexturedTri, Model, ObjectTexture, RoomQuad,
		RoomTri, Sprite, SpriteSequence, SpriteTexture, StaticMesh, ATLAS_PIXELS, PALETTE_LEN,
	}, tr2, tr3, Readable,
};
use crate::as_bytes::ReinterpretAsBytes;

pub trait RoomVertex: ReinterpretAsBytes {
	fn pos(&self) -> I16Vec3;
}

pub trait RoomGeom {
	type RoomVertex: RoomVertex;
	fn vertices(&self) -> &[Self::RoomVertex];
	fn quads(&self) -> &[RoomQuad];
	fn tris(&self) -> &[RoomTri];
	fn sprites(&self) -> &[Sprite];
}

pub trait RoomStaticMesh {
	fn static_mesh_id(&self) -> u16;
	fn pos(&self) -> IVec3;
	fn angle(&self) -> u16;
}

pub trait Room {
	type RoomGeom<'a>: RoomGeom where Self: 'a;
	type RoomStaticMesh: RoomStaticMesh;
	fn pos(&self) -> IVec3;
	fn get_geom(&self) -> Self::RoomGeom<'_>;
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh];
	fn alt_room_index(&self) -> u16;
}

pub trait Entity {
	fn room_index(&self) -> u16;
	fn model_id(&self) -> u16;
	fn pos(&self) -> IVec3;
	fn angle(&self) -> u16;
}

pub trait SolidFace: ReinterpretAsBytes + Copy {
	fn color_index_24bit(&self) -> u8;
	fn color_index_32bit(&self) -> u8;
}

pub trait Mesh {
	type SolidQuad: SolidFace;
	type SolidTri: SolidFace;
	fn vertices(&self) -> &[I16Vec3];
	fn textured_quads(&self) -> &[MeshTexturedQuad];
	fn textured_tris(&self) -> &[MeshTexturedTri];
	fn solid_quads(&self) -> &[Self::SolidQuad];
	fn solid_tris(&self) -> &[Self::SolidTri];
}

pub trait Frame {
	fn offset(&self) -> I16Vec3;
	fn iter_rotations(&self) -> impl Iterator<Item = Mat4>;
}

pub trait Level: Readable {
	type Room: Room;
	type Entity: Entity;
	type Mesh<'a>: Mesh where Self: 'a;
	type Frame<'a>: Frame where Self: 'a;
	fn static_meshes(&self) -> &[StaticMesh];
	fn models(&self) -> &[Model];
	fn sprite_sequences(&self) -> &[SpriteSequence];
	fn rooms(&self) -> &[Self::Room];
	fn entities(&self) -> &[Self::Entity];
	fn object_textures(&self) -> &[ObjectTexture];
	fn sprite_textures(&self) -> &[SpriteTexture];
	fn mesh_offsets(&self) -> &[u32];
	fn palette(&self) -> &[Color24Bit; PALETTE_LEN];
	fn atlases(&self) -> &[[u8; ATLAS_PIXELS]];
	fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode];
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_>;
	fn get_frame(&self, model: &Model) -> Self::Frame<'_>;
}

//impl helpers

fn to_radians(angle: u16) -> f32 {
	angle as f32 / 1024.0 * TAU
}

fn to_mat(angles: U16Vec3) -> Mat4 {
	let [x, y, z] = angles.to_array().map(to_radians);
	Mat4::from_rotation_y(y) * Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z)
}

//impls

impl RoomVertex for tr1::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl<'a> RoomGeom for tr1::RoomGeom<'a> {
	type RoomVertex = tr1::RoomVertex;
	fn vertices(&self) -> &[Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &[RoomQuad] { self.quads }
	fn tris(&self) -> &[RoomTri] { self.tris }
	fn sprites(&self) -> &[Sprite] { self.sprites }
}

impl RoomStaticMesh for tr1::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr1::Room {
	type RoomGeom<'a> = tr1::RoomGeom<'a>;
	type RoomStaticMesh = tr1::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn get_geom(&self) -> Self::RoomGeom<'_> { self.get_geom() }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn alt_room_index(&self) -> u16 { self.alt_room_index }
}

impl Entity for tr1::Entity {
	fn room_index(&self) -> u16 { self.room_index }
	fn model_id(&self) -> u16 { self.model_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl SolidFace for tr1::MeshSolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> u8 { (self.color_index >> 8) as u8 }
}

impl SolidFace for tr1::MeshSolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> u8 { (self.color_index >> 8) as u8 }
}

impl<'a> Mesh for tr1::Mesh<'a> {
	type SolidQuad = tr1::MeshSolidQuad;
	type SolidTri = tr1::MeshSolidTri;
	fn vertices(&self) -> &[I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &[MeshTexturedQuad] { self.textured_quads }
	fn textured_tris(&self) -> &[MeshTexturedTri] { self.textured_tris }
	fn solid_quads(&self) -> &[Self::SolidQuad] { self.solid_quads }
	fn solid_tris(&self) -> &[Self::SolidTri] { self.solid_tris }
}

impl Frame for &tr1::Frame {
	fn offset(&self) -> I16Vec3 { self.offset }
	fn iter_rotations(&self) -> impl Iterator<Item = Mat4> {
		self.rotations.iter().map(|rot| to_mat(rot.get_angles()))
	}
}

impl Level for tr1::Level {
	type Room = tr1::Room;
	type Entity = tr1::Entity;
	type Mesh<'a> = tr1::Mesh<'a>;
	type Frame<'a> = &'a tr1::Frame;
	fn static_meshes(&self) -> &[StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[Model] { &self.models }
	fn sprite_sequences(&self) -> &[SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette(&self) -> &[Color24Bit; PALETTE_LEN] { &self.palette }
	fn atlases(&self) -> &[[u8; ATLAS_PIXELS]] { &self.atlases }
	fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Model) -> Self::Frame<'_> { self.get_frame(model) }
}

impl RoomVertex for tr2::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl<'a> RoomGeom for tr2::RoomGeom<'a> {
	type RoomVertex = tr2::RoomVertex;
	fn vertices(&self) -> &[Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &[RoomQuad] { self.quads }
	fn tris(&self) -> &[RoomTri] { self.tris }
	fn sprites(&self) -> &[Sprite] { self.sprites }
}

impl RoomStaticMesh for tr2::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr2::Room {
	type RoomGeom<'a> = tr2::RoomGeom<'a>;
	type RoomStaticMesh = tr2::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn get_geom(&self) -> Self::RoomGeom<'_> { self.get_geom() }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn alt_room_index(&self) -> u16 { self.alt_room_index }
}

impl Entity for tr2::Entity {
	fn room_index(&self) -> u16 { self.room_index }
	fn model_id(&self) -> u16 { self.model_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl SolidFace for tr2::MeshSolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> u8 { self.color_index_32bit }
}

impl SolidFace for tr2::MeshSolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> u8 { self.color_index_32bit }
}

impl<'a> Mesh for tr2::Mesh<'a> {
	type SolidQuad = tr2::MeshSolidQuad;
	type SolidTri = tr2::MeshSolidTri;
	fn vertices(&self) -> &[I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &[MeshTexturedQuad] { self.textured_quads }
	fn textured_tris(&self) -> &[MeshTexturedTri] { self.textured_tris }
	fn solid_quads(&self) -> &[Self::SolidQuad] { self.solid_quads }
	fn solid_tris(&self) -> &[Self::SolidTri] { self.solid_tris }
}

impl<'a> Frame for tr2::Frame<'a> {
	fn offset(&self) -> I16Vec3 { self.frame_data.offset }
	fn iter_rotations(&self) -> impl Iterator<Item = Mat4> {
		self.iter_rotations().map(|rot| {
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
		})
	}
}

impl Level for tr2::Level {
	type Room = tr2::Room;
	type Entity = tr2::Entity;
	type Mesh<'a> = tr2::Mesh<'a>;
	type Frame<'a> = tr2::Frame<'a>;
	fn static_meshes(&self) -> &[StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[Model] { &self.models }
	fn sprite_sequences(&self) -> &[SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette(&self) -> &[Color24Bit; PALETTE_LEN] { &self.palette_24bit }
	fn atlases(&self) -> &[[u8; ATLAS_PIXELS]] { &self.atlases.atlases_palette }
	fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Model) -> Self::Frame<'_> { self.get_frame(model) }
}

impl RoomVertex for tr3::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl<'a> RoomGeom for tr3::RoomGeom<'a> {
	type RoomVertex = tr3::RoomVertex;
	fn vertices(&self) -> &[Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &[RoomQuad] { self.quads }
	fn tris(&self) -> &[RoomTri] { self.tris }
	fn sprites(&self) -> &[Sprite] { self.sprites }
}

impl RoomStaticMesh for tr3::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr3::Room {
	type RoomGeom<'a> = tr3::RoomGeom<'a>;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn get_geom(&self) -> Self::RoomGeom<'_> { self.get_geom() }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn alt_room_index(&self) -> u16 { self.alt_room_index }
}

impl Level for tr3::Level {
	type Room = tr3::Room;
	type Entity = tr2::Entity;
	type Mesh<'a> = tr2::Mesh<'a>;
	type Frame<'a> = tr2::Frame<'a>;
	fn static_meshes(&self) -> &[StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[Model] { &self.models }
	fn sprite_sequences(&self) -> &[SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette(&self) -> &[Color24Bit; PALETTE_LEN] { &self.palette_24bit }
	fn atlases(&self) -> &[[u8; ATLAS_PIXELS]] { &self.atlases.atlases_palette }
	fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Model) -> Self::Frame<'_> { self.get_frame(model) }
}
