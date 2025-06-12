use std::{f32::consts::TAU, iter};
use glam::{I16Vec3, IVec3, Mat4, U16Vec2, U16Vec3, Vec3};
use glam_traits::GVec3;
use tr_model::{tr1, tr2, tr3, tr4, tr5};

const SINGLE_ANGLE_DIVISOR_TR2: f32 = (tr2::SINGLE_ANGLE_MASK + 1) as f32;
const SINGLE_ANGLE_DIVISOR_TR4: f32 = (tr4::SINGLE_ANGLE_MASK + 1) as f32;
const MULTI_ANGLE_DIVISOR: f32 = 1024.0;

pub enum LevelStore {
	Tr1(tr1::Level),
	Tr2(tr2::Level),
	Tr3(tr3::Level),
	Tr4(tr4::Level),
	Tr5(tr5::Level),
}

pub struct Layer<'a, V, Q, T> {
	pub vertices: &'a [V],
	pub quads: &'a [Q],
	pub tris: &'a [T],
}

pub trait Model {
	fn id(&self) -> u32;
	fn mesh_offset_index(&self) -> u16;
	fn num_meshes(&self) -> u16;
}

pub trait RoomVertex {
	type Pos: GVec3;
	fn pos(&self) -> Self::Pos;
}

pub trait Face<const N: usize> {
	fn vertex_indices(&self) -> [u16; N];
}

pub trait TexturedFace<const N: usize>: Face<N> {
	fn object_texture_index(&self) -> u16;
}

pub trait RoomFace<const N: usize>: TexturedFace<N> {
	fn double_sided(&self) -> bool;
}

pub trait MeshTexturedFace<const N: usize>: TexturedFace<N> {
	fn additive(&self) -> bool;
}

pub trait SolidFace<const N: usize>: Face<N> {
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
	type RoomQuad: RoomFace<4>;
	type RoomTri: RoomFace<3>;
	type RoomStaticMesh: RoomStaticMesh;
	fn pos(&self) -> IVec3;
	fn vertices(&self) -> &[Self::RoomVertex];
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>>;
	fn sprites(&self) -> &[tr1::Sprite];
	fn num_sectors(&self) -> tr1::NumSectors;
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

// #[allow(dead_code)]//todo: remove
pub trait ObjectTexture {
	const UVS_OFFSET: u32;
	fn blend_mode(&self) -> u16;
	fn atlas_index(&self) -> u16;
	fn uvs(&self) -> [U16Vec2; 4];
}

pub trait Mesh<'a> {
	type TexturedQuad: MeshTexturedFace<4>;
	type TexturedTri: MeshTexturedFace<3>;
	type SolidQuad: SolidFace<4>;
	type SolidTri: SolidFace<3>;
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

pub trait Level {
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
	fn store(self) -> LevelStore;
}

//impl helpers

fn to_radians(angle: u16, divisor: f32) -> f32 {
	angle as f32 / divisor * TAU
}

fn angles_to_mat(angles: U16Vec3) -> Mat4 {
	let [x, y, z] = angles.to_array().map(|a| to_radians(a, MULTI_ANGLE_DIVISOR));
	Mat4::from_rotation_y(y) * Mat4::from_rotation_x(x) * Mat4::from_rotation_z(z)
}

fn angle_to_mat(axis: tr2::Axis, angle: u16, divisor: f32) -> Mat4 {
	let angle = to_radians(angle, divisor);
	match axis {
		tr2::Axis::X => Mat4::from_rotation_x(angle),
		tr2::Axis::Y => Mat4::from_rotation_y(angle),
		tr2::Axis::Z => Mat4::from_rotation_z(angle),
	}
}

//impls

//tr1

impl Model for tr1::Model {
	fn id(&self) -> u32 { self.id }
	fn mesh_offset_index(&self) -> u16 { self.mesh_offset_index }
	fn num_meshes(&self) -> u16 { self.num_meshes }
}

impl RoomVertex for tr1::RoomVertex {
	type Pos = I16Vec3;
	fn pos(&self) -> Self::Pos { self.pos }
}

impl Face<4> for tr1::TexturedQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr1::TexturedTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl TexturedFace<4> for tr1::TexturedQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace<3> for tr1::TexturedTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl RoomFace<4> for tr1::TexturedQuad {
	fn double_sided(&self) -> bool { false }
}

impl RoomFace<3> for tr1::TexturedTri {
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
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let layer = Layer {
			vertices: &self.vertices,
			quads: &self.quads,
			tris: &self.tris,
		};
		iter::once(layer)
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn num_sectors(&self) -> tr1::NumSectors { self.num_sectors }
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

impl Face<4> for tr1::SolidQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr1::SolidTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl SolidFace<4> for tr1::SolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl SolidFace<3> for tr1::SolidTri {
	fn color_index_24bit(&self) -> u8 { self.color_index as u8 }
	fn color_index_32bit(&self) -> Option<u8> { None }
}

impl MeshTexturedFace<4> for tr1::TexturedQuad {
	fn additive(&self) -> bool { false }
}

impl MeshTexturedFace<3> for tr1::TexturedTri {
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
		self.rotations.iter().map(|rot| angles_to_mat(rot.get_angles()))
	}
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
	fn store(self) -> LevelStore { LevelStore::Tr1(self) }
}

//tr2

impl RoomVertex for tr2::RoomVertex {
	type Pos = I16Vec3;
	fn pos(&self) -> Self::Pos { self.pos }
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
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let layer = Layer {
			vertices: &self.vertices,
			quads: &self.quads,
			tris: &self.tris,
		};
		iter::once(layer)
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn num_sectors(&self) -> tr1::NumSectors { self.num_sectors }
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

impl Face<4> for tr2::SolidQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr2::SolidTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl SolidFace<4> for tr2::SolidQuad {
	fn color_index_24bit(&self) -> u8 { self.color_index_24bit }
	fn color_index_32bit(&self) -> Option<u8> { Some(self.color_index_32bit) }
}

impl SolidFace<3> for tr2::SolidTri {
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
		fn to_mat(rot: tr2::FrameRotation) -> Mat4 {
			match rot {
				tr2::FrameRotation::AllAxes(angles) => angles_to_mat(angles),
				tr2::FrameRotation::SingleAxis(axis, angle) => {
					angle_to_mat(axis, angle, SINGLE_ANGLE_DIVISOR_TR2)
				},
			}
		}
		self.iter_rotations().map(to_mat)
	}
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
	fn store(self) -> LevelStore { LevelStore::Tr2(self) }
}

//tr3

impl RoomVertex for tr3::RoomVertex {
	type Pos = I16Vec3;
	fn pos(&self) -> Self::Pos { self.pos }
}

impl Face<4> for tr3::DsQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr3::DsTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl TexturedFace<4> for tr3::DsQuad {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl TexturedFace<3> for tr3::DsTri {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace<4> for tr3::DsQuad {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl RoomFace<3> for tr3::DsTri {
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
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let layer = Layer {
			vertices: &self.vertices,
			quads: &self.quads,
			tris: &self.tris,
		};
		iter::once(layer)
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn num_sectors(&self) -> tr1::NumSectors { self.num_sectors }
	fn room_static_meshes(&self) -> &[Self::RoomStaticMesh] { &self.room_static_meshes }
	fn flip_room_index(&self) -> u16 { self.flip_room_index }
	fn flip_group(&self) -> u8 { 0 }
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
	fn store(self) -> LevelStore { LevelStore::Tr3(self) }
}

//tr4

impl Room for tr4::Room {
	type RoomVertex = tr3::RoomVertex;
	type RoomQuad = tr3::DsQuad;
	type RoomTri = tr3::DsTri;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { IVec3::new(self.x, 0, self.z) }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let layer = Layer {
			vertices: &self.vertices,
			quads: &self.quads,
			tris: &self.tris,
		};
		iter::once(layer)
	}
	fn sprites(&self) -> &[tr1::Sprite] { &self.sprites }
	fn num_sectors(&self) -> tr1::NumSectors { self.num_sectors }
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

impl Face<4> for tr4::EffectsQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr4::EffectsTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl TexturedFace<4> for tr4::EffectsQuad {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl TexturedFace<3> for tr4::EffectsTri {
	fn object_texture_index(&self) -> u16 { self.object_texture_index }
}

impl MeshTexturedFace<4> for tr4::EffectsQuad {
	fn additive(&self) -> bool { self.flags.additive() }
}

impl MeshTexturedFace<3> for tr4::EffectsTri {
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
		fn to_mat(rot: tr4::FrameRotation) -> Mat4 {
			match rot {
				tr4::FrameRotation::AllAxes(angles) => angles_to_mat(angles),
				tr4::FrameRotation::SingleAxis(axis, angle) => {
					angle_to_mat(axis, angle, SINGLE_ANGLE_DIVISOR_TR4)
				},
			}
		}
		self.iter_rotations().map(to_mat)
	}
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
	fn store(self) -> LevelStore { LevelStore::Tr4(self) }
}

//tr5

impl Model for tr5::Model {
	fn id(&self) -> u32 { self.id }
	fn mesh_offset_index(&self) -> u16 { self.mesh_offset_index }
	fn num_meshes(&self) -> u16 { self.num_meshes }
}

impl RoomVertex for tr5::RoomVertex {
	type Pos = Vec3;
	fn pos(&self) -> Self::Pos { self.pos }
}

impl Face<4> for tr5::EffectsQuad {
	fn vertex_indices(&self) -> [u16; 4] { self.vertex_indices }
}

impl Face<3> for tr5::EffectsTri {
	fn vertex_indices(&self) -> [u16; 3] { self.vertex_indices }
}

impl TexturedFace<4> for tr5::EffectsQuad {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl TexturedFace<3> for tr5::EffectsTri {
	fn object_texture_index(&self) -> u16 { self.texture.object_texture_index() }
}

impl RoomFace<4> for tr5::EffectsQuad {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl RoomFace<3> for tr5::EffectsTri {
	fn double_sided(&self) -> bool { self.texture.double_sided() }
}

impl Room for tr5::Room {
	type RoomVertex = tr5::RoomVertex;
	type RoomQuad = tr5::EffectsQuad;
	type RoomTri = tr5::EffectsTri;
	type RoomStaticMesh = tr3::RoomStaticMesh;
	fn pos(&self) -> IVec3 { self.pos1 }
	fn vertices(&self) -> &[Self::RoomVertex] { &self.vertices }
	fn layers(&self) -> impl Iterator<Item = Layer<Self::RoomVertex, Self::RoomQuad, Self::RoomTri>> {
		let mut vertex_count = 0;
		let get_layer = move |index: usize| {
			let vertex_offset = vertex_count;
			vertex_count += self.layers[index].num_vertices as usize;
			let layer_faces = &self.layer_faces[index];
			Layer {
				vertices: &self.vertices[vertex_offset..vertex_count],
				quads: &layer_faces.quads,
				tris: &layer_faces.tris,
			}
		};
		(0..self.layers.len()).map(get_layer)
	}
	fn sprites(&self) -> &[tr1::Sprite] { &[] }
	fn num_sectors(&self) -> tr1::NumSectors { self.num_sectors }
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
	fn store(self) -> LevelStore { LevelStore::Tr5(self) }
}
