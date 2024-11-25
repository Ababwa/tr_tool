use std::f32::consts::TAU;
use glam::{I16Vec3, IVec3, Mat4, U16Vec3};
use tr_model::{tr1, tr2, tr3, Readable};
use crate::as_bytes::ReinterpretAsBytes;

pub enum LevelStore {
	Tr1(Box<tr1::Level>),
	Tr2(Box<tr2::Level>),
	Tr3(Box<tr3::Level>),
}

#[derive(Clone, Copy, Debug)]
pub enum RoomFaceType {
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

pub trait Vertex: ReinterpretAsBytes {}

pub trait RoomVertex: Vertex {
	fn pos(&self) -> I16Vec3;
}

pub trait Face: ReinterpretAsBytes {}

pub trait TexturedFace: Face {
	fn object_texture_index(&self) -> u16;
}

pub trait RoomFace: TexturedFace {
	const TYPE: RoomFaceType;
	fn double_sided(&self) -> bool;
}

pub trait MeshFace: Face {
	const TYPE: MeshFaceType;
}

pub trait RoomGeom<'a> {
	type RoomVertex: RoomVertex;
	type RoomQuad: RoomFace;
	type RoomTri: RoomFace;
	fn vertices(&self) -> &'a [Self::RoomVertex];
	fn quads(&self) -> &'a [Self::RoomQuad];
	fn tris(&self) -> &'a [Self::RoomTri];
	fn sprites(&self) -> &'a [tr1::Sprite];
}

pub trait RoomStaticMesh {
	fn static_mesh_id(&self) -> u16;
	fn pos(&self) -> IVec3;
	fn angle(&self) -> u16;
}

pub trait Room {
	type RoomGeom<'a>: RoomGeom<'a> where Self: 'a;
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

pub trait SolidFace {
	fn color_index_24bit(&self) -> u8;
	fn color_index_32bit(&self) -> Option<u8>;
}

pub trait Mesh<'a> {
	type TexturedQuad: MeshFace + TexturedFace;
	type TexturedTri: MeshFace + TexturedFace;
	type SolidQuad: MeshFace + SolidFace;
	type SolidTri: MeshFace + SolidFace;
	fn vertices(&self) -> &'a [I16Vec3];
	fn textured_quads(&self) -> &'a [Self::TexturedQuad];
	fn textured_tris(&self) -> &'a [Self::TexturedTri];
	fn solid_quads(&self) -> &'a [Self::SolidQuad];
	fn solid_tris(&self) -> &'a [Self::SolidTri];
}

pub trait Frame {
	fn offset(&self) -> I16Vec3;
	fn iter_rotations(&self) -> impl Iterator<Item = Mat4>;
}

pub trait Level: Readable {
	type Room: Room;
	type Entity: Entity;
	type Mesh<'a>: Mesh<'a> where Self: 'a;
	type Frame<'a>: Frame where Self: 'a;
	fn static_meshes(&self) -> &[tr1::StaticMesh];
	fn models(&self) -> &[tr1::Model];
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence];
	fn rooms(&self) -> &[Self::Room];
	fn entities(&self) -> &[Self::Entity];
	fn object_textures(&self) -> &[tr1::ObjectTexture];
	fn sprite_textures(&self) -> &[tr1::SpriteTexture];
	fn mesh_offsets(&self) -> &[u32];
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]>;
	fn palette_32bit(&self) -> Option<&[tr2::Color32Bit; tr1::PALETTE_LEN]>;
	fn atlases(&self) -> &[[u8; tr1::ATLAS_PIXELS]];
	fn get_mesh_nodes(&self, model: &tr1::Model) -> &[tr1::MeshNode];
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_>;
	fn get_frame(&self, model: &tr1::Model) -> Self::Frame<'_>;
	fn store(self: Box<Self>) -> LevelStore;
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

impl Vertex for I16Vec3 {}

impl Vertex for tr1::RoomVertex {}

impl RoomVertex for tr1::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl Face for tr1::RoomQuad {}
impl Face for tr1::RoomTri {}
impl Face for tr1::MeshTexturedQuad {}
impl Face for tr1::MeshTexturedTri {}
impl Face for tr1::MeshSolidQuad {}
impl Face for tr1::MeshSolidTri {}

impl TexturedFace for tr1::RoomQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace for tr1::RoomTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl RoomFace for tr1::RoomQuad {
	const TYPE: RoomFaceType = RoomFaceType::Quad;
	fn double_sided(&self) -> bool { false }
}

impl RoomFace for tr1::RoomTri {
	const TYPE: RoomFaceType = RoomFaceType::Tri;
	fn double_sided(&self) -> bool { false }
}

impl<'a> RoomGeom<'a> for tr1::RoomGeom<'a> {
	type RoomVertex = tr1::RoomVertex;
	type RoomQuad = tr1::RoomQuad;
	type RoomTri = tr1::RoomTri;
	fn vertices(&self) -> &'a [Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &'a [Self::RoomQuad] { self.quads }
	fn tris(&self) -> &'a [Self::RoomTri] { self.tris }
	fn sprites(&self) -> &'a [tr1::Sprite] { self.sprites }
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

impl MeshFace for tr1::MeshTexturedQuad { const TYPE: MeshFaceType = MeshFaceType::TexturedQuad; }
impl MeshFace for tr1::MeshTexturedTri { const TYPE: MeshFaceType = MeshFaceType::TexturedTri; }
impl MeshFace for tr1::MeshSolidQuad { const TYPE: MeshFaceType = MeshFaceType::SolidQuad; }
impl MeshFace for tr1::MeshSolidTri { const TYPE: MeshFaceType = MeshFaceType::SolidTri; }

impl SolidFace for tr1::MeshSolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl SolidFace for tr1::MeshSolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl TexturedFace for tr1::MeshTexturedQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace for tr1::MeshTexturedTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl<'a> Mesh<'a> for tr1::Mesh<'a> {
	type TexturedQuad = tr1::MeshTexturedQuad;
	type TexturedTri = tr1::MeshTexturedTri;
	type SolidQuad = tr1::MeshSolidQuad;
	type SolidTri = tr1::MeshSolidTri;
	fn vertices(&self) -> &'a [I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &'a [Self::TexturedQuad] { self.textured_quads }
	fn textured_tris(&self) -> &'a [Self::TexturedTri] { self.textured_tris }
	fn solid_quads(&self) -> &'a [Self::SolidQuad] { self.solid_quads }
	fn solid_tris(&self) -> &'a [Self::SolidTri] { self.solid_tris }
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
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[tr1::Model] { &self.models }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[tr1::ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32Bit; tr1::PALETTE_LEN]> { None }
	fn atlases(&self) -> &[[u8; tr1::ATLAS_PIXELS]] { &self.atlases }
	fn get_mesh_nodes(&self, model: &tr1::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &tr1::Model) -> Self::Frame<'_> { self.get_frame(model) }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr1(self) }
}

impl Vertex for tr2::RoomVertex {}

impl RoomVertex for tr2::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl<'a> RoomGeom<'a> for tr2::RoomGeom<'a> {
	type RoomVertex = tr2::RoomVertex;
	type RoomQuad = tr1::RoomQuad;
	type RoomTri = tr1::RoomTri;
	fn vertices(&self) -> &'a [Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &'a [Self::RoomQuad] { self.quads }
	fn tris(&self) -> &'a [Self::RoomTri] { self.tris }
	fn sprites(&self) -> &'a [tr1::Sprite] { self.sprites }
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

impl Face for tr2::MeshSolidQuad {}
impl Face for tr2::MeshSolidTri {}

impl MeshFace for tr2::MeshSolidQuad {const TYPE: MeshFaceType = MeshFaceType::SolidQuad; }
impl MeshFace for tr2::MeshSolidTri {const TYPE: MeshFaceType = MeshFaceType::SolidTri; }

impl SolidFace for tr2::MeshSolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> Option<u8> { Some(self.color_index_32bit) }
}

impl SolidFace for tr2::MeshSolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> Option<u8> { Some(self.color_index_32bit) }
}

impl<'a> Mesh<'a> for tr2::Mesh<'a> {
	type TexturedQuad = tr1::MeshTexturedQuad;
	type TexturedTri = tr1::MeshTexturedTri;
	type SolidQuad = tr2::MeshSolidQuad;
	type SolidTri = tr2::MeshSolidTri;
	fn vertices(&self) -> &'a [I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &'a [tr1::MeshTexturedQuad] { self.textured_quads }
	fn textured_tris(&self) -> &'a [tr1::MeshTexturedTri] { self.textured_tris }
	fn solid_quads(&self) -> &'a [Self::SolidQuad] { self.solid_quads }
	fn solid_tris(&self) -> &'a [Self::SolidTri] { self.solid_tris }
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
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[tr1::Model] { &self.models }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[tr1::ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette_24bit) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32Bit; tr1::PALETTE_LEN]> { Some(&self.palette_32bit) }
	fn atlases(&self) -> &[[u8; tr1::ATLAS_PIXELS]] { &self.atlases.atlases_palette }
	fn get_mesh_nodes(&self, model: &tr1::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &tr1::Model) -> Self::Frame<'_> { self.get_frame(model) }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr2(self) }
}

impl Vertex for tr3::RoomVertex {}

impl RoomVertex for tr3::RoomVertex {
	fn pos(&self) -> I16Vec3 { self.pos }
}

impl Face for tr3::RoomQuad {}
impl Face for tr3::RoomTri {}

impl TexturedFace for tr3::RoomQuad {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl TexturedFace for tr3::RoomTri {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace for tr3::RoomQuad {
	const TYPE: RoomFaceType = RoomFaceType::Quad;
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl RoomFace for tr3::RoomTri {
	const TYPE: RoomFaceType = RoomFaceType::Tri;
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl<'a> RoomGeom<'a> for tr3::RoomGeom<'a> {
	type RoomVertex = tr3::RoomVertex;
	type RoomQuad = tr3::RoomQuad;
	type RoomTri = tr3::RoomTri;
	fn vertices(&self) -> &'a [Self::RoomVertex] { self.vertices }
	fn quads(&self) -> &'a [Self::RoomQuad] { self.quads }
	fn tris(&self) -> &'a [Self::RoomTri] { self.tris }
	fn sprites(&self) -> &'a [tr1::Sprite] { self.sprites }
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
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn models(&self) -> &[tr1::Model] { &self.models }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[tr1::ObjectTexture] { &self.object_textures }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette_24bit) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32Bit; tr1::PALETTE_LEN]> { Some(&self.palette_32bit) }
	fn atlases(&self) -> &[[u8; tr1::ATLAS_PIXELS]] { &self.atlases.atlases_palette }
	fn get_mesh_nodes(&self, model: &tr1::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &tr1::Model) -> Self::Frame<'_> { self.get_frame(model) }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr3(self) }
}
