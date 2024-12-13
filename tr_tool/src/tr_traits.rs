use std::f32::consts::TAU;
use glam::{I16Vec3, IVec3, Mat4, U16Vec2, U16Vec3, Vec3};
use tr_model::{tr1, tr2, tr3, tr4, tr5, Readable};
use crate::{as_bytes::ReinterpretAsBytes, object_data::PolyType};

pub enum LevelStore {
	Tr1(Box<tr1::Level>),
	Tr2(Box<tr2::Level>),
	Tr3(Box<tr3::Level>),
	Tr4(Box<tr4::Level>),
	Tr5(Box<tr5::Level>),
}

impl LevelStore {
	pub fn as_dyn(&self) -> &dyn LevelDyn {
		match self {
			LevelStore::Tr1(level) => level.as_ref(),
			LevelStore::Tr2(level) => level.as_ref(),
			LevelStore::Tr3(level) => level.as_ref(),
			LevelStore::Tr4(level) => level.as_ref(),
			LevelStore::Tr5(level) => level.as_ref(),
		}
	}
}

pub struct RoomGeom<'a, V, Q, T> {
	pub vertices: &'a [V],
	pub quads: &'a [Q],
	pub tris: &'a [T],
}

pub trait Model {
	fn id(&self) -> u32;
	fn mesh_offset_index(&self) -> u16;
	fn num_meshes(&self) -> u16;
}

pub trait RoomVertex: ReinterpretAsBytes {
	fn pos(&self) -> Vec3;
}

pub trait Face: ReinterpretAsBytes {
	const POLY_TYPE: PolyType;
}

pub trait TexturedFace: Face {
	fn object_texture_index(&self) -> u16;
}

pub trait RoomFace: TexturedFace {
	fn double_sided(&self) -> bool;
}

pub trait MeshTexturedFace: TexturedFace {
	fn additive(&self) -> bool;
}

pub trait SolidFace: Face {
	fn color_index_24bit(&self) -> u8;
	fn color_index_32bit(&self) -> Option<u8>;
}

pub trait RoomStaticMesh {
	fn static_mesh_id(&self) -> u16;
	fn pos(&self) -> IVec3;
	fn angle(&self) -> u16;
}

pub trait Room {
	type RoomVertex: RoomVertex;
	type RoomQuad: RoomFace;
	type RoomTri: RoomFace;
	type RoomStaticMesh: RoomStaticMesh;
	fn pos(&self) -> IVec3;
	fn vertices(&self) -> &[Self::RoomVertex];
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>>;
	fn sprites(&self) -> &[tr1::Sprite];
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh];
	fn flip_room_index(&self) -> u16;
	fn flip_group(&self) -> u8;
}

pub trait Entity {
	fn room_index(&self) -> u16;
	fn model_id(&self) -> u16;
	fn pos(&self) -> IVec3;
	fn angle(&self) -> u16;
}

#[allow(dead_code)]//todo: remove
pub trait ObjectTexture: ReinterpretAsBytes {
	const UVS_OFFSET: u32;
	fn blend_mode(&self) -> u16;
	fn atlas_index(&self) -> u16;
	fn uvs(&self) -> [U16Vec2; 4];
}

pub trait Mesh<'a> {
	type TexturedQuad: MeshTexturedFace;
	type TexturedTri: MeshTexturedFace;
	type SolidQuad: SolidFace;
	type SolidTri: SolidFace;
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

pub trait LevelDyn {
	fn static_meshes(&self) -> &[tr1::StaticMesh];
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence];
	fn sprite_textures(&self) -> &[tr1::SpriteTexture];
	fn mesh_offsets(&self) -> &[u32];
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]>;
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]>;
	fn num_atlases(&self) -> usize;
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]>;
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]>;
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]>;
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]>;
	fn store(self: Box<Self>) -> LevelStore;
}

pub trait Level: LevelDyn + Readable {
	type Model: Model;
	type Room: Room;
	type Entity: Entity;
	type ObjectTexture: ObjectTexture;
	type Mesh<'a>: Mesh<'a> where Self: 'a;
	type Frame<'a>: Frame where Self: 'a;
	fn models(&self) -> &[Self::Model];
	fn rooms(&self) -> &[Self::Room];
	fn entities(&self) -> &[Self::Entity];
	fn object_textures(&self) -> &[Self::ObjectTexture];
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode];
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_>;
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_>;
}

//impl helpers

fn to_radians(angle: u16, divisor: f32) -> f32 {
	angle as f32 / divisor * TAU
}

fn to_mat(angles: U16Vec3) -> Mat4 {
	let [x, y, z] = angles.to_array().map(|a| to_radians(a, 1024.0));
	Mat4::from_rotation_y(y) * Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z)
}

//impls

//tr1

impl Model for tr1::Model {
	fn id(&self) -> u32 { self.id }
	fn mesh_offset_index(&self) -> u16 { self.mesh_offset_index }
	fn num_meshes(&self) -> u16 { self.num_meshes }
}

impl RoomVertex for tr1::RoomVertex {
	fn pos(&self) -> Vec3 { self.pos.as_vec3() }
}

impl Face for tr1::TexturedQuad { const POLY_TYPE: PolyType = PolyType::Quad; }
impl Face for tr1::TexturedTri { const POLY_TYPE: PolyType = PolyType::Tri; }

impl TexturedFace for tr1::TexturedQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace for tr1::TexturedTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl RoomFace for tr1::TexturedQuad {
	fn double_sided(&self) -> bool { false }
}

impl RoomFace for tr1::TexturedTri {
	fn double_sided(&self) -> bool { false }
}

impl RoomStaticMesh for tr1::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr1::Room {
	type RoomVertex = tr1::RoomVertex;
	type RoomQuad = tr1::TexturedQuad;
	type RoomTri = tr1::TexturedTri;
	type RoomStaticMesh = tr1::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		[RoomGeom { vertices: &self.vertices, quads: &self.quads, tris: &self.tris }]
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { 0 }
}

impl Entity for tr1::Entity {
	fn room_index(&self) -> u16 { self.room_index }
	fn model_id(&self) -> u16 { self.model_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl ObjectTexture for tr1::ObjectTexture {
	const UVS_OFFSET: u32 = 2;
	fn blend_mode(&self) -> u16 { self.blend_mode }
	fn atlas_index(&self) -> u16 { self.atlas_index }
	fn uvs(&self) -> [U16Vec2; 4] { self.uvs }
}

impl Face for tr1::SolidQuad { const POLY_TYPE: PolyType = PolyType::Quad; }
impl Face for tr1::SolidTri { const POLY_TYPE: PolyType = PolyType::Tri; }

impl SolidFace for tr1::SolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl SolidFace for tr1::SolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl MeshTexturedFace for tr1::TexturedQuad {
	fn additive(&self) -> bool { false }
}

impl MeshTexturedFace for tr1::TexturedTri {
	fn additive(&self) -> bool { false }
}

impl<'a> Mesh<'a> for tr1::Mesh<'a> {
	type TexturedQuad = tr1::TexturedQuad;
	type TexturedTri = tr1::TexturedTri;
	type SolidQuad = tr1::SolidQuad;
	type SolidTri = tr1::SolidTri;
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

impl LevelDyn for tr1::Level {
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]> { None }
	fn num_atlases(&self) -> usize { self.atlases.len() }
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]> { Some(&self.atlases) }
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]> { None }
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr1(self) }
}

impl Level for tr1::Level {
	type Model = tr1::Model;
	type Room = tr1::Room;
	type Entity = tr1::Entity;
	type ObjectTexture = tr1::ObjectTexture;
	type Mesh<'a> = tr1::Mesh<'a>;
	type Frame<'a> = &'a tr1::Frame;
	fn models(&self) -> &[Self::Model] { &self.models }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[Self::ObjectTexture] { &self.object_textures }
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_> { self.get_frame(model) }
}

//tr2

impl RoomVertex for tr2::RoomVertex {
	fn pos(&self) -> Vec3 { self.pos.as_vec3() }
}

impl RoomStaticMesh for tr2::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr2::Room {
	type RoomVertex = tr2::RoomVertex;
	type RoomQuad = tr1::TexturedQuad;
	type RoomTri = tr1::TexturedTri;
	type RoomStaticMesh = tr2::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		[RoomGeom { vertices: &self.vertices, quads: &self.quads, tris: &self.tris }]
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { 0 }
}

impl Entity for tr2::Entity {
	fn room_index(&self) -> u16 { self.room_index }
	fn model_id(&self) -> u16 { self.model_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Face for tr2::SolidQuad { const POLY_TYPE: PolyType = PolyType::Quad; }
impl Face for tr2::SolidTri { const POLY_TYPE: PolyType = PolyType::Tri; }

impl SolidFace for tr2::SolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> Option<u8> { Some(self.color_index_32bit) }
}

impl SolidFace for tr2::SolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> Option<u8> { Some(self.color_index_32bit) }
}

impl<'a> Mesh<'a> for tr2::Mesh<'a> {
	type TexturedQuad = tr1::TexturedQuad;
	type TexturedTri = tr1::TexturedTri;
	type SolidQuad = tr2::SolidQuad;
	type SolidTri = tr2::SolidTri;
	fn vertices(&self) -> &'a [I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &'a [Self::TexturedQuad] { self.textured_quads }
	fn textured_tris(&self) -> &'a [Self::TexturedTri] { self.textured_tris }
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
					let angle = to_radians(angle, 1024.0);
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

impl LevelDyn for tr2::Level {
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette_24bit) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]> { Some(&self.palette_32bit) }
	fn num_atlases(&self) -> usize { self.atlases_palette.len() }
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]> { Some(&self.atlases_palette) }
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_16bit)
	}
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr2(self) }
}

impl Level for tr2::Level {
	type Model = tr1::Model;
	type Room = tr2::Room;
	type Entity = tr2::Entity;
	type ObjectTexture = tr1::ObjectTexture;
	type Mesh<'a> = tr2::Mesh<'a>;
	type Frame<'a> = tr2::Frame<'a>;
	fn models(&self) -> &[Self::Model] { &self.models }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[Self::ObjectTexture] { &self.object_textures }
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_> { self.get_frame(model) }
}

//tr3

impl RoomVertex for tr3::RoomVertex {
	fn pos(&self) -> Vec3 { self.pos.as_vec3() }
}

impl Face for tr3::DsQuad { const POLY_TYPE: PolyType = PolyType::Quad; }
impl Face for tr3::DsTri { const POLY_TYPE: PolyType = PolyType::Tri; }

impl TexturedFace for tr3::DsQuad {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl TexturedFace for tr3::DsTri {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace for tr3::DsQuad {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl RoomFace for tr3::DsTri {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl RoomStaticMesh for tr3::RoomStaticMesh {
	fn static_mesh_id(&self) -> u16 { self.static_mesh_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl Room for tr3::Room {
	type RoomVertex = tr3::RoomVertex;
	type RoomQuad = tr3::DsQuad;
	type RoomTri = tr3::DsTri;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		[RoomGeom { vertices: &self.vertices, quads: &self.quads, tris: &self.tris }]
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { 0 }
}

impl LevelDyn for tr3::Level {
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { Some(&self.palette_24bit) }
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]> { Some(&self.palette_32bit) }
	fn num_atlases(&self) -> usize { self.atlases_palette.len() }
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]> { Some(&self.atlases_palette) }
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_16bit)
	}
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> { None }
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr3(self) }
}

impl Level for tr3::Level {
	type Model = tr1::Model;
	type Room = tr3::Room;
	type Entity = tr2::Entity;
	type ObjectTexture = tr1::ObjectTexture;
	type Mesh<'a> = tr2::Mesh<'a>;
	type Frame<'a> = tr2::Frame<'a>;
	fn models(&self) -> &[Self::Model] { &self.models }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[Self::ObjectTexture] { &self.object_textures }
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_> { self.get_frame(model) }
}

//tr4

impl Room for tr4::Room {
	type RoomVertex = tr3::RoomVertex;
	type RoomQuad = tr3::DsQuad;
	type RoomTri = tr3::DsTri;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		[RoomGeom { vertices: &self.vertices, quads: &self.quads, tris: &self.tris }]
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { self.flip_group }
}

impl Entity for tr4::Entity {
	fn room_index(&self) -> u16 { self.room_index }
	fn model_id(&self) -> u16 { self.model_id }
	fn pos(&self) -> IVec3 { self.pos }
	fn angle(&self) -> u16 { self.angle }
}

impl ObjectTexture for tr4::ObjectTexture {
	const UVS_OFFSET: u32 = 3;
	fn blend_mode(&self) -> u16 { self.blend_mode }
	fn atlas_index(&self) -> u16 { self.atlas_index_face_type.atlas_index() }
	fn uvs(&self) -> [U16Vec2; 4] { self.uvs }
}

impl Face for tr4::EffectsQuad { const POLY_TYPE: PolyType = PolyType::Quad; }
impl Face for tr4::EffectsTri { const POLY_TYPE: PolyType = PolyType::Tri; }

impl TexturedFace for tr4::EffectsQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace for tr4::EffectsTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl MeshTexturedFace for tr4::EffectsQuad {
	fn additive(&self) -> bool { self.flags.additive() }
}

impl MeshTexturedFace for tr4::EffectsTri {
	fn additive(&self) -> bool { self.flags.additive() }
}

impl<'a> Mesh<'a> for tr4::Mesh<'a> {
	type TexturedQuad = tr4::EffectsQuad;
	type TexturedTri = tr4::EffectsTri;
	type SolidQuad = tr1::SolidQuad;//hacky
	type SolidTri = tr1::SolidTri;
	fn vertices(&self) -> &'a [I16Vec3] { self.vertices }
	fn textured_quads(&self) -> &'a [Self::TexturedQuad] { self.quads }
	fn textured_tris(&self) -> &'a [Self::TexturedTri] { self.tris }
	fn solid_quads(&self) -> &'a [Self::SolidQuad] { &[] }
	fn solid_tris(&self) -> &'a [Self::SolidTri] { &[] }
}

impl<'a> Frame for tr4::Frame<'a> {
	fn offset(&self) -> I16Vec3 { self.frame_data.offset }
	fn iter_rotations(&self) -> impl Iterator<Item = Mat4> {
		self.iter_rotations().map(|rot| {
			match rot {
				tr4::FrameRotation::AllAxes(angles) => to_mat(angles),
				tr4::FrameRotation::SingleAxis(axis, angle) => {
					let angle = to_radians(angle, 4096.0);
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

impl LevelDyn for tr4::Level {
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.level_data.static_meshes }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.level_data.sprite_sequences }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.level_data.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.level_data.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { None }
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]> { None }
	fn num_atlases(&self) -> usize { self.atlases_32bit.len() }
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]> { None }
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_16bit)
	}
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_32bit)
	}
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> {
		Some(&self.misc_images[..])
	}
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr4(self) }
}

impl Level for tr4::Level {
	type Model = tr1::Model;
	type Room = tr4::Room;
	type Entity = tr4::Entity;
	type ObjectTexture = tr4::ObjectTexture;
	type Mesh<'a> = tr4::Mesh<'a>;
	type Frame<'a> = tr4::Frame<'a>;
	fn models(&self) -> &[Self::Model] { &self.level_data.models }
	fn rooms(&self) -> &[Self::Room] { &self.level_data.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.level_data.entities }
	fn object_textures(&self) -> &[Self::ObjectTexture] { &self.level_data.object_textures }
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_> { self.get_frame(model) }
}

//tr5

impl Model for tr5::Model {
	fn id(&self) -> u32 { self.id }
	fn mesh_offset_index(&self) -> u16 { self.mesh_offset_index }
	fn num_meshes(&self) -> u16 { self.num_meshes }
}

impl RoomVertex for tr5::RoomVertex {
	fn pos(&self) -> Vec3 { self.pos }
}

impl Face for tr5::EffectsQuad {
	const POLY_TYPE: PolyType = PolyType::Quad;
}

impl TexturedFace for tr5::EffectsQuad {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace for tr5::EffectsQuad {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl Face for tr5::EffectsTri {
	const POLY_TYPE: PolyType = PolyType::Tri;
}

impl TexturedFace for tr5::EffectsTri {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace for tr5::EffectsTri {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl Room for tr5::Room {
	type RoomVertex = tr5::RoomVertex;
	type RoomQuad = tr5::EffectsQuad;
	type RoomTri = tr5::EffectsTri;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { self.pos1 }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn geom(&self) -> impl IntoIterator<Item = RoomGeom<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let mut vertex_offset = 0;
		self.layers.iter().enumerate().map(move |(index, layer)| {
			let offset = vertex_offset;
			vertex_offset += layer.num_vertices;
			RoomGeom {
				vertices: &self.vertices[offset as usize..][..layer.num_vertices as usize],
				quads: &self.layer_faces[index].quads,
				tris: &self.layer_faces[index].tris,
			}
		})
	}
	fn sprites(&self) -> &[tr1::Sprite] { &[] }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { self.flip_group }
}

impl ObjectTexture for tr5::ObjectTexture {
	const UVS_OFFSET: u32 = 3;
	fn blend_mode(&self) -> u16 { self.blend_mode }
	fn atlas_index(&self) -> u16 { self.atlas_index_face_type.atlas_index() }
	fn uvs(&self) -> [U16Vec2; 4] { self.uvs }
}

impl LevelDyn for tr5::Level {
	fn static_meshes(&self) -> &[tr1::StaticMesh] { &self.static_meshes }
	fn sprite_sequences(&self) -> &[tr1::SpriteSequence] { &self.sprite_sequences }
	fn sprite_textures(&self) -> &[tr1::SpriteTexture] { &self.sprite_textures }
	fn mesh_offsets(&self) -> &[u32] { &self.mesh_offsets }
	fn palette_24bit(&self) -> Option<&[tr1::Color24Bit; tr1::PALETTE_LEN]> { None }
	fn palette_32bit(&self) -> Option<&[tr2::Color32BitRgb; tr1::PALETTE_LEN]> { None }
	fn num_atlases(&self) -> usize { self.atlases_32bit.len() }
	fn atlases_palette(&self) -> Option<&[[u8; tr1::ATLAS_PIXELS]]> { None }
	fn atlases_16bit(&self) -> Option<&[[tr2::Color16BitArgb; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_16bit)
	}
	fn atlases_32bit(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> {
		Some(&self.atlases_32bit)
	}
	fn misc_images(&self) -> Option<&[[tr4::Color32BitBgra; tr1::ATLAS_PIXELS]]> {
		Some(&self.misc_images[..])
	}
	fn store(self: Box<Self>) -> LevelStore { LevelStore::Tr5(self) }
}

impl Level for tr5::Level {
	type Model = tr5::Model;
	type Room = tr5::Room;
	type Entity = tr4::Entity;
	type ObjectTexture = tr5::ObjectTexture;
	type Mesh<'a> = tr4::Mesh<'a>;
	type Frame<'a> = tr4::Frame<'a>;
	fn models(&self) -> &[Self::Model] { &self.models }
	fn rooms(&self) -> &[Self::Room] { &self.rooms }
	fn entities(&self) -> &[Self::Entity] { &self.entities }
	fn object_textures(&self) -> &[Self::ObjectTexture] { &self.object_textures }
	fn get_mesh_nodes(&self, model: &Self::Model) -> &[tr1::MeshNode] { self.get_mesh_nodes(model) }
	fn get_mesh(&self, mesh_offset: u32) -> Self::Mesh<'_> { self.get_mesh(mesh_offset) }
	fn get_frame(&self, model: &Self::Model) -> Self::Frame<'_> { self.get_frame(model) }
}
