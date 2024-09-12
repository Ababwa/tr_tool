use std::prelude::rust_2021::*;
extern crate std;
pub use tr_readable::Readable;
pub mod tr1 {
    use std::io::{Cursor, Error, ErrorKind, Read, Result};
    use bitfield::bitfield;
    use byteorder::{ReadBytesExt, LE};
    use glam::{I16Vec2, I16Vec3, IVec3, U16Vec2, U16Vec3};
    use glam_traits::ext::U8Vec2;
    use nonmax::{NonMaxU16, NonMaxU8};
    use shared::MinMax;
    use tr_readable::{read_boxed_slice, read_list, Readable};
    pub const IMAGE_SIZE: usize = 256;
    pub const NUM_PIXELS: usize = IMAGE_SIZE * IMAGE_SIZE;
    pub const PALETTE_SIZE: usize = 256;
    pub const SOUND_MAP_SIZE: usize = 256;
    pub const LIGHT_MAP_SIZE: usize = 32;
    pub const ZONE_MULT: usize = 6;
    pub struct RoomVertex {
        /// Relative to Room
        pub vertex: I16Vec3,
        pub light: u16,
    }
    impl tr_readable::Readable for RoomVertex {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let vertex = tr_readable::Readable::read(reader).unwrap();
            let light = tr_readable::Readable::read(reader).unwrap();
            Ok(RoomVertex { vertex, light })
        }
    }
    impl ::core::clone::Clone for RoomVertex {
        fn clone(&self) -> RoomVertex {
            *self
        }
    }
    impl ::core::marker::Copy for RoomVertex {}
    pub struct Face<const N: usize, D: Readable> {
        pub vertex_indices: [u16; N],
        pub details: D,
    }
    impl<const N: usize, D: Readable> tr_readable::Readable for Face<N, D> {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let vertex_indices = tr_readable::Readable::read(reader).unwrap();
            let details = tr_readable::Readable::read(reader).unwrap();
            Ok(Face { vertex_indices, details })
        }
    }
    impl<const N: usize, D: ::core::clone::Clone + Readable> ::core::clone::Clone
    for Face<N, D> {
        fn clone(&self) -> Face<N, D> {
            Face {
                vertex_indices: ::core::clone::Clone::clone(&self.vertex_indices),
                details: ::core::clone::Clone::clone(&self.details),
            }
        }
    }
    impl<const N: usize, D: ::core::marker::Copy + Readable> ::core::marker::Copy
    for Face<N, D> {}
    impl<T> ::bitfield::BitRange<T> for TexturedFaceDetails
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for TexturedFaceDetails
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct TexturedFaceDetails(pub u16);
    impl tr_readable::Readable for TexturedFaceDetails {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(TexturedFaceDetails(field0))
        }
    }
    impl ::core::clone::Clone for TexturedFaceDetails {
        fn clone(&self) -> TexturedFaceDetails {
            *self
        }
    }
    impl ::core::marker::Copy for TexturedFaceDetails {}
    impl TexturedFaceDetails {
        /// Index into object_textures
        pub fn texture_index(&self) -> u16 {
            use ::bitfield::BitRange;
            let raw_value: u16 = self.bit_range(14, 0);
            ::bitfield::Into::into(raw_value)
        }
        pub fn double_sided(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(15)
        }
    }
    pub type TexturedTri = Face<3, TexturedFaceDetails>;
    pub type TexturedQuad = Face<4, TexturedFaceDetails>;
    pub struct SolidFaceDetails {
        /// Index into palette3
        pub palette3_index: u8,
        /// Index into palette4
        pub palette4_index: u8,
    }
    impl tr_readable::Readable for SolidFaceDetails {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let palette3_index = tr_readable::Readable::read(reader).unwrap();
            let palette4_index = tr_readable::Readable::read(reader).unwrap();
            Ok(SolidFaceDetails {
                palette3_index,
                palette4_index,
            })
        }
    }
    impl ::core::clone::Clone for SolidFaceDetails {
        fn clone(&self) -> SolidFaceDetails {
            *self
        }
    }
    impl ::core::marker::Copy for SolidFaceDetails {}
    pub type SolidTri = Face<3, SolidFaceDetails>;
    pub type SolidQuad = Face<4, SolidFaceDetails>;
    pub struct Sprite {
        /// Index into Room.vertices
        pub vertex_index: u16,
        /// Index into sprite_textures
        pub texture_index: u16,
    }
    impl tr_readable::Readable for Sprite {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let vertex_index = tr_readable::Readable::read(reader).unwrap();
            let texture_index = tr_readable::Readable::read(reader).unwrap();
            Ok(Sprite {
                vertex_index,
                texture_index,
            })
        }
    }
    impl ::core::clone::Clone for Sprite {
        fn clone(&self) -> Sprite {
            *self
        }
    }
    impl ::core::marker::Copy for Sprite {}
    pub struct Portal {
        /// Index into rooms
        pub adjoining_room_index: u16,
        pub normal: I16Vec3,
        /// Relative to Room
        pub vertices: [I16Vec3; 4],
    }
    impl tr_readable::Readable for Portal {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let adjoining_room_index = tr_readable::Readable::read(reader).unwrap();
            let normal = tr_readable::Readable::read(reader).unwrap();
            let vertices = tr_readable::Readable::read(reader).unwrap();
            Ok(Portal {
                adjoining_room_index,
                normal,
                vertices,
            })
        }
    }
    impl ::core::clone::Clone for Portal {
        fn clone(&self) -> Portal {
            *self
        }
    }
    impl ::core::marker::Copy for Portal {}
    impl<T> ::bitfield::BitRange<T> for SectorMaterialAndBox
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for SectorMaterialAndBox
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct SectorMaterialAndBox(pub u16);
    impl tr_readable::Readable for SectorMaterialAndBox {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(SectorMaterialAndBox(field0))
        }
    }
    impl ::core::clone::Clone for SectorMaterialAndBox {
        fn clone(&self) -> SectorMaterialAndBox {
            *self
        }
    }
    impl ::core::marker::Copy for SectorMaterialAndBox {}
    impl SectorMaterialAndBox {
        /// Footstep sound
        pub fn material(&self) -> u16 {
            use ::bitfield::BitRange;
            let raw_value: u16 = self.bit_range(3, 0);
            ::bitfield::Into::into(raw_value)
        }
        /// Index into BoxData.boxes
        pub fn box_index(&self) -> u16 {
            use ::bitfield::BitRange;
            let raw_value: u16 = self.bit_range(14, 4);
            ::bitfield::Into::into(raw_value)
        }
    }
    pub struct Sector {
        /// Index into floor_data
        pub floor_data_index: u16,
        pub material_and_box: SectorMaterialAndBox,
        /// Index into rooms
        pub room_below_id: Option<NonMaxU8>,
        pub floor: i8,
        /// Index into rooms
        pub room_above_index: Option<NonMaxU8>,
        pub ceiling: i8,
    }
    impl tr_readable::Readable for Sector {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let floor_data_index = tr_readable::Readable::read(reader).unwrap();
            let material_and_box = tr_readable::Readable::read(reader).unwrap();
            let room_below_id = tr_readable::Readable::read(reader).unwrap();
            let floor = tr_readable::Readable::read(reader).unwrap();
            let room_above_index = tr_readable::Readable::read(reader).unwrap();
            let ceiling = tr_readable::Readable::read(reader).unwrap();
            Ok(Sector {
                floor_data_index,
                material_and_box,
                room_below_id,
                floor,
                room_above_index,
                ceiling,
            })
        }
    }
    impl ::core::clone::Clone for Sector {
        fn clone(&self) -> Sector {
            *self
        }
    }
    impl ::core::marker::Copy for Sector {}
    pub struct Sectors {
        pub num_sectors: U16Vec2,
        pub sectors: Box<[Sector]>,
    }
    impl ::core::clone::Clone for Sectors {
        fn clone(&self) -> Sectors {
            Sectors {
                num_sectors: ::core::clone::Clone::clone(&self.num_sectors),
                sectors: ::core::clone::Clone::clone(&self.sectors),
            }
        }
    }
    impl Readable for Sectors {
        fn read<R: Read>(reader: &mut R) -> Result<Self> {
            let num_sectors = U16Vec2::read(reader)?;
            let sectors = read_boxed_slice(
                reader,
                num_sectors.element_product() as usize,
            )?;
            Ok(Self { num_sectors, sectors })
        }
    }
    pub struct Light {
        pub pos: IVec3,
        pub brightness: u16,
        pub fallout: u32,
    }
    impl tr_readable::Readable for Light {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let pos = tr_readable::Readable::read(reader).unwrap();
            let brightness = tr_readable::Readable::read(reader).unwrap();
            let fallout = tr_readable::Readable::read(reader).unwrap();
            Ok(Light { pos, brightness, fallout })
        }
    }
    impl ::core::clone::Clone for Light {
        fn clone(&self) -> Light {
            *self
        }
    }
    impl ::core::marker::Copy for Light {}
    pub struct RoomStaticMesh {
        /// World coords
        pub pos: IVec3,
        /// Units are 1/65536th of a rotation
        pub rotation: u16,
        pub color: u16,
        /// Id into LevelData.static_meshes
        pub static_mesh_id: u16,
    }
    impl tr_readable::Readable for RoomStaticMesh {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let pos = tr_readable::Readable::read(reader).unwrap();
            let rotation = tr_readable::Readable::read(reader).unwrap();
            let color = tr_readable::Readable::read(reader).unwrap();
            let static_mesh_id = tr_readable::Readable::read(reader).unwrap();
            Ok(RoomStaticMesh {
                pos,
                rotation,
                color,
                static_mesh_id,
            })
        }
    }
    impl ::core::clone::Clone for RoomStaticMesh {
        fn clone(&self) -> RoomStaticMesh {
            *self
        }
    }
    impl ::core::marker::Copy for RoomStaticMesh {}
    impl<T> ::bitfield::BitRange<T> for RoomFlags
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for RoomFlags
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct RoomFlags(pub u16);
    impl tr_readable::Readable for RoomFlags {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(RoomFlags(field0))
        }
    }
    impl ::core::clone::Clone for RoomFlags {
        fn clone(&self) -> RoomFlags {
            *self
        }
    }
    impl ::core::marker::Copy for RoomFlags {}
    impl RoomFlags {
        pub fn water(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(0)
        }
    }
    pub struct Room {
        /// World coord
        pub x: i32,
        /// World coord
        pub z: i32,
        pub y_bottom: i32,
        pub y_top: i32,
        pub vertices: Box<[RoomVertex]>,
        /// `vertex_indices` index into Room.vertices
        pub quads: Box<[TexturedQuad]>,
        /// `vertex_indices` index into Room.vertices
        pub tris: Box<[TexturedTri]>,
        pub sprites: Box<[Sprite]>,
        pub portals: Box<[Portal]>,
        pub sectors: Sectors,
        pub ambient_light: u16,
        pub lights: Box<[Light]>,
        pub room_static_meshes: Box<[RoomStaticMesh]>,
        /// Index into LevelData.rooms
        pub flip_room_index: Option<NonMaxU16>,
        pub flags: RoomFlags,
    }
    impl tr_readable::Readable for Room {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let x = tr_readable::Readable::read(reader).unwrap();
            let z = tr_readable::Readable::read(reader).unwrap();
            let y_bottom = tr_readable::Readable::read(reader).unwrap();
            let y_top = tr_readable::Readable::read(reader).unwrap();
            tr_readable::skip(reader, 4)?;
            let vertices = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let quads = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let tris = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let sprites = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let portals = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let sectors = tr_readable::Readable::read(reader).unwrap();
            let ambient_light = tr_readable::Readable::read(reader).unwrap();
            let lights = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let room_static_meshes = tr_readable::read_list::<_, _, u16>(reader)
                .unwrap();
            let flip_room_index = tr_readable::Readable::read(reader).unwrap();
            let flags = tr_readable::Readable::read(reader).unwrap();
            Ok(Room {
                x,
                z,
                y_bottom,
                y_top,
                vertices,
                quads,
                tris,
                sprites,
                portals,
                sectors,
                ambient_light,
                lights,
                room_static_meshes,
                flip_room_index,
                flags,
            })
        }
    }
    impl ::core::clone::Clone for Room {
        fn clone(&self) -> Room {
            Room {
                x: ::core::clone::Clone::clone(&self.x),
                z: ::core::clone::Clone::clone(&self.z),
                y_bottom: ::core::clone::Clone::clone(&self.y_bottom),
                y_top: ::core::clone::Clone::clone(&self.y_top),
                vertices: ::core::clone::Clone::clone(&self.vertices),
                quads: ::core::clone::Clone::clone(&self.quads),
                tris: ::core::clone::Clone::clone(&self.tris),
                sprites: ::core::clone::Clone::clone(&self.sprites),
                portals: ::core::clone::Clone::clone(&self.portals),
                sectors: ::core::clone::Clone::clone(&self.sectors),
                ambient_light: ::core::clone::Clone::clone(&self.ambient_light),
                lights: ::core::clone::Clone::clone(&self.lights),
                room_static_meshes: ::core::clone::Clone::clone(
                    &self.room_static_meshes,
                ),
                flip_room_index: ::core::clone::Clone::clone(&self.flip_room_index),
                flags: ::core::clone::Clone::clone(&self.flags),
            }
        }
    }
    pub struct Animation {
        /// Byte offset into frame_data
        pub frame_byte_offset: u32,
        /// 30ths of a second
        pub frame_duration: u8,
        pub num_frames: u8,
        pub state: u16,
        /// Fixed-point
        pub speed: u32,
        /// Fixed-point
        pub accel: u32,
        pub frame_start: u16,
        pub frame_end: u16,
        pub next_anim: u16,
        pub next_frame: u16,
        pub num_state_changes: u16,
        /// Id? into state_changes
        pub state_change_id: u16,
        pub num_anim_commands: u16,
        /// Id? into anim_commands
        pub anim_command_id: u16,
    }
    impl tr_readable::Readable for Animation {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let frame_byte_offset = tr_readable::Readable::read(reader).unwrap();
            let frame_duration = tr_readable::Readable::read(reader).unwrap();
            let num_frames = tr_readable::Readable::read(reader).unwrap();
            let state = tr_readable::Readable::read(reader).unwrap();
            let speed = tr_readable::Readable::read(reader).unwrap();
            let accel = tr_readable::Readable::read(reader).unwrap();
            let frame_start = tr_readable::Readable::read(reader).unwrap();
            let frame_end = tr_readable::Readable::read(reader).unwrap();
            let next_anim = tr_readable::Readable::read(reader).unwrap();
            let next_frame = tr_readable::Readable::read(reader).unwrap();
            let num_state_changes = tr_readable::Readable::read(reader).unwrap();
            let state_change_id = tr_readable::Readable::read(reader).unwrap();
            let num_anim_commands = tr_readable::Readable::read(reader).unwrap();
            let anim_command_id = tr_readable::Readable::read(reader).unwrap();
            Ok(Animation {
                frame_byte_offset,
                frame_duration,
                num_frames,
                state,
                speed,
                accel,
                frame_start,
                frame_end,
                next_anim,
                next_frame,
                num_state_changes,
                state_change_id,
                num_anim_commands,
                anim_command_id,
            })
        }
    }
    impl ::core::clone::Clone for Animation {
        fn clone(&self) -> Animation {
            *self
        }
    }
    impl ::core::marker::Copy for Animation {}
    pub struct StateChange {
        pub state: u16,
        pub num_anim_dispatches: u16,
        /// Id? into LevelData.anim_dispatches
        pub anim_dispatch_id: u16,
    }
    impl tr_readable::Readable for StateChange {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let state = tr_readable::Readable::read(reader).unwrap();
            let num_anim_dispatches = tr_readable::Readable::read(reader).unwrap();
            let anim_dispatch_id = tr_readable::Readable::read(reader).unwrap();
            Ok(StateChange {
                state,
                num_anim_dispatches,
                anim_dispatch_id,
            })
        }
    }
    impl ::core::clone::Clone for StateChange {
        fn clone(&self) -> StateChange {
            *self
        }
    }
    impl ::core::marker::Copy for StateChange {}
    pub struct AnimDispatch {
        pub low_frame: u16,
        pub high_frame: u16,
        /// Id? into LevelData.animations
        pub next_anim_id: u16,
        /// Id? into LevelData.frames
        pub next_frame_id: u16,
    }
    impl tr_readable::Readable for AnimDispatch {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let low_frame = tr_readable::Readable::read(reader).unwrap();
            let high_frame = tr_readable::Readable::read(reader).unwrap();
            let next_anim_id = tr_readable::Readable::read(reader).unwrap();
            let next_frame_id = tr_readable::Readable::read(reader).unwrap();
            Ok(AnimDispatch {
                low_frame,
                high_frame,
                next_anim_id,
                next_frame_id,
            })
        }
    }
    impl ::core::clone::Clone for AnimDispatch {
        fn clone(&self) -> AnimDispatch {
            *self
        }
    }
    impl ::core::marker::Copy for AnimDispatch {}
    pub struct Model {
        pub id: u32,
        pub num_meshes: u16,
        /// Id into meshes
        pub mesh_id: u16,
        /// Offset into mesh_node_data
        pub mesh_node_offset: u32,
        /// Byte offset into frames
        pub frame_byte_offset: u32,
        /// Index into animations
        pub anim_index: Option<NonMaxU16>,
    }
    impl tr_readable::Readable for Model {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let id = tr_readable::Readable::read(reader).unwrap();
            let num_meshes = tr_readable::Readable::read(reader).unwrap();
            let mesh_id = tr_readable::Readable::read(reader).unwrap();
            let mesh_node_offset = tr_readable::Readable::read(reader).unwrap();
            let frame_byte_offset = tr_readable::Readable::read(reader).unwrap();
            let anim_index = tr_readable::Readable::read(reader).unwrap();
            Ok(Model {
                id,
                num_meshes,
                mesh_id,
                mesh_node_offset,
                frame_byte_offset,
                anim_index,
            })
        }
    }
    impl ::core::clone::Clone for Model {
        fn clone(&self) -> Model {
            *self
        }
    }
    impl ::core::marker::Copy for Model {}
    pub struct BoundBox {
        pub x: MinMax<i16>,
        pub y: MinMax<i16>,
        pub z: MinMax<i16>,
    }
    impl tr_readable::Readable for BoundBox {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let x = tr_readable::Readable::read(reader).unwrap();
            let y = tr_readable::Readable::read(reader).unwrap();
            let z = tr_readable::Readable::read(reader).unwrap();
            Ok(BoundBox { x, y, z })
        }
    }
    impl ::core::clone::Clone for BoundBox {
        fn clone(&self) -> BoundBox {
            *self
        }
    }
    impl ::core::marker::Copy for BoundBox {}
    pub struct StaticMesh {
        pub id: u32,
        /// Id into LevelData.meshes
        pub mesh_id: u16,
        pub visibility: BoundBox,
        pub collision: BoundBox,
        pub flags: u16,
    }
    impl tr_readable::Readable for StaticMesh {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let id = tr_readable::Readable::read(reader).unwrap();
            let mesh_id = tr_readable::Readable::read(reader).unwrap();
            let visibility = tr_readable::Readable::read(reader).unwrap();
            let collision = tr_readable::Readable::read(reader).unwrap();
            let flags = tr_readable::Readable::read(reader).unwrap();
            Ok(StaticMesh {
                id,
                mesh_id,
                visibility,
                collision,
                flags,
            })
        }
    }
    impl ::core::clone::Clone for StaticMesh {
        fn clone(&self) -> StaticMesh {
            *self
        }
    }
    impl ::core::marker::Copy for StaticMesh {}
    pub enum BlendMode {
        Opaque,
        Test,
        Add,
    }
    impl ::core::clone::Clone for BlendMode {
        fn clone(&self) -> BlendMode {
            *self
        }
    }
    impl ::core::marker::Copy for BlendMode {}
    impl Readable for BlendMode {
        fn read<R: Read>(reader: &mut R) -> Result<Self> {
            Ok(
                match reader.read_u16::<LE>()? {
                    0 => BlendMode::Opaque,
                    1 => BlendMode::Test,
                    2 => BlendMode::Add,
                    _ => {
                        return Err(
                            Error::new(ErrorKind::InvalidData, "invalid blend mode"),
                        );
                    }
                },
            )
        }
    }
    impl<T> ::bitfield::BitRange<T> for ObjectTextureAtlasAndTriangle
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for ObjectTextureAtlasAndTriangle
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct ObjectTextureAtlasAndTriangle(pub u16);
    impl tr_readable::Readable for ObjectTextureAtlasAndTriangle {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(ObjectTextureAtlasAndTriangle(field0))
        }
    }
    impl ::core::clone::Clone for ObjectTextureAtlasAndTriangle {
        fn clone(&self) -> ObjectTextureAtlasAndTriangle {
            *self
        }
    }
    impl ::core::marker::Copy for ObjectTextureAtlasAndTriangle {}
    impl ObjectTextureAtlasAndTriangle {
        /// Index into images
        pub fn atlas_index(&self) -> u16 {
            use ::bitfield::BitRange;
            let raw_value: u16 = self.bit_range(14, 0);
            ::bitfield::Into::into(raw_value)
        }
        pub fn triangle(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(15)
        }
    }
    pub struct ObjectTexture {
        pub blend_mode: BlendMode,
        pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
        /// Units are 1/256th of a pixel
        pub vertices: [U16Vec2; 4],
    }
    impl tr_readable::Readable for ObjectTexture {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let blend_mode = tr_readable::Readable::read(reader).unwrap();
            let atlas_and_triangle = tr_readable::Readable::read(reader).unwrap();
            let vertices = tr_readable::Readable::read(reader).unwrap();
            Ok(ObjectTexture {
                blend_mode,
                atlas_and_triangle,
                vertices,
            })
        }
    }
    impl ::core::clone::Clone for ObjectTexture {
        fn clone(&self) -> ObjectTexture {
            *self
        }
    }
    impl ::core::marker::Copy for ObjectTexture {}
    pub struct SpriteTexture {
        /// Index into images
        pub atlas_index: u16,
        pub pos: U8Vec2,
        pub size: U16Vec2,
        pub world_bounds: [I16Vec2; 2],
    }
    impl tr_readable::Readable for SpriteTexture {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let atlas_index = tr_readable::Readable::read(reader).unwrap();
            let pos = tr_readable::Readable::read(reader).unwrap();
            let size = tr_readable::Readable::read(reader).unwrap();
            let world_bounds = tr_readable::Readable::read(reader).unwrap();
            Ok(SpriteTexture {
                atlas_index,
                pos,
                size,
                world_bounds,
            })
        }
    }
    impl ::core::clone::Clone for SpriteTexture {
        fn clone(&self) -> SpriteTexture {
            *self
        }
    }
    impl ::core::marker::Copy for SpriteTexture {}
    pub struct SpriteSequence {
        pub id: u32,
        pub neg_length: i16,
        /// Index into sprite_textures
        pub sprite_index: u16,
    }
    impl tr_readable::Readable for SpriteSequence {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let id = tr_readable::Readable::read(reader).unwrap();
            let neg_length = tr_readable::Readable::read(reader).unwrap();
            let sprite_index = tr_readable::Readable::read(reader).unwrap();
            Ok(SpriteSequence {
                id,
                neg_length,
                sprite_index,
            })
        }
    }
    impl ::core::clone::Clone for SpriteSequence {
        fn clone(&self) -> SpriteSequence {
            *self
        }
    }
    impl ::core::marker::Copy for SpriteSequence {}
    pub struct Camera {
        /// World coords
        pub pos: IVec3,
        /// Index into LevelData.rooms
        pub room_index: u16,
        pub flags: u16,
    }
    impl tr_readable::Readable for Camera {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let pos = tr_readable::Readable::read(reader).unwrap();
            let room_index = tr_readable::Readable::read(reader).unwrap();
            let flags = tr_readable::Readable::read(reader).unwrap();
            Ok(Camera { pos, room_index, flags })
        }
    }
    impl ::core::clone::Clone for Camera {
        fn clone(&self) -> Camera {
            *self
        }
    }
    impl ::core::marker::Copy for Camera {}
    pub struct SoundSource {
        /// World coords
        pub pos: IVec3,
        pub sound_id: u16,
        pub flags: u16,
    }
    impl tr_readable::Readable for SoundSource {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let pos = tr_readable::Readable::read(reader).unwrap();
            let sound_id = tr_readable::Readable::read(reader).unwrap();
            let flags = tr_readable::Readable::read(reader).unwrap();
            Ok(SoundSource {
                pos,
                sound_id,
                flags,
            })
        }
    }
    impl ::core::clone::Clone for SoundSource {
        fn clone(&self) -> SoundSource {
            *self
        }
    }
    impl ::core::marker::Copy for SoundSource {}
    impl<T> ::bitfield::BitRange<T> for OverlapIndex
    where
        u16: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for OverlapIndex
    where
        u16: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct OverlapIndex(pub u16);
    impl tr_readable::Readable for OverlapIndex {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(OverlapIndex(field0))
        }
    }
    impl ::core::clone::Clone for OverlapIndex {
        fn clone(&self) -> OverlapIndex {
            *self
        }
    }
    impl ::core::marker::Copy for OverlapIndex {}
    impl OverlapIndex {
        pub fn index(&self) -> u16 {
            use ::bitfield::BitRange;
            let raw_value: u16 = self.bit_range(13, 0);
            ::bitfield::Into::into(raw_value)
        }
        pub fn blocked(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(14)
        }
        pub fn blockable(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(15)
        }
    }
    pub struct TrBox {
        /// Sectors
        pub z: MinMax<u32>,
        pub x: MinMax<u32>,
        pub y: i16,
        pub overlap: OverlapIndex,
    }
    impl tr_readable::Readable for TrBox {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let z = tr_readable::Readable::read(reader).unwrap();
            let x = tr_readable::Readable::read(reader).unwrap();
            let y = tr_readable::Readable::read(reader).unwrap();
            let overlap = tr_readable::Readable::read(reader).unwrap();
            Ok(TrBox { z, x, y, overlap })
        }
    }
    impl ::core::clone::Clone for TrBox {
        fn clone(&self) -> TrBox {
            *self
        }
    }
    impl ::core::marker::Copy for TrBox {}
    pub struct BoxData {
        pub boxes: Box<[TrBox]>,
        pub overlap_data: Box<[u16]>,
        pub zone_data: Box<[u16]>,
    }
    impl ::core::clone::Clone for BoxData {
        fn clone(&self) -> BoxData {
            BoxData {
                boxes: ::core::clone::Clone::clone(&self.boxes),
                overlap_data: ::core::clone::Clone::clone(&self.overlap_data),
                zone_data: ::core::clone::Clone::clone(&self.zone_data),
            }
        }
    }
    impl Readable for BoxData {
        fn read<R: Read>(reader: &mut R) -> Result<Self> {
            let boxes = read_list::<_, _, u32>(reader)?;
            let overlap_data = read_list::<_, _, u32>(reader)?;
            let zone_data = read_boxed_slice(reader, boxes.len() * ZONE_MULT)?;
            Ok(Self {
                boxes,
                overlap_data,
                zone_data,
            })
        }
    }
    pub struct Entity {
        /// Id into models or sprite_textures
        pub model_id: u16,
        /// Index into rooms
        pub room_index: u16,
        /// World coords
        pub pos: IVec3,
        /// Units are 1/65536th of a rotation
        pub rotation: u16,
        /// If None, use mesh light
        pub brightness: Option<NonMaxU16>,
        pub flags: u16,
    }
    impl tr_readable::Readable for Entity {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let model_id = tr_readable::Readable::read(reader).unwrap();
            let room_index = tr_readable::Readable::read(reader).unwrap();
            let pos = tr_readable::Readable::read(reader).unwrap();
            let rotation = tr_readable::Readable::read(reader).unwrap();
            let brightness = tr_readable::Readable::read(reader).unwrap();
            let flags = tr_readable::Readable::read(reader).unwrap();
            Ok(Entity {
                model_id,
                room_index,
                pos,
                rotation,
                brightness,
                flags,
            })
        }
    }
    impl ::core::clone::Clone for Entity {
        fn clone(&self) -> Entity {
            *self
        }
    }
    impl ::core::marker::Copy for Entity {}
    pub struct Color3 {
        pub r: u8,
        pub g: u8,
        pub b: u8,
    }
    impl tr_readable::Readable for Color3 {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let r = tr_readable::Readable::read(reader).unwrap();
            let g = tr_readable::Readable::read(reader).unwrap();
            let b = tr_readable::Readable::read(reader).unwrap();
            Ok(Color3 { r, g, b })
        }
    }
    impl ::core::clone::Clone for Color3 {
        fn clone(&self) -> Color3 {
            *self
        }
    }
    impl ::core::marker::Copy for Color3 {}
    pub struct CinematicFrame {
        pub target: I16Vec3,
        pub pos: I16Vec3,
        pub fov: i16,
        pub roll: i16,
    }
    impl tr_readable::Readable for CinematicFrame {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let target = tr_readable::Readable::read(reader).unwrap();
            let pos = tr_readable::Readable::read(reader).unwrap();
            let fov = tr_readable::Readable::read(reader).unwrap();
            let roll = tr_readable::Readable::read(reader).unwrap();
            Ok(CinematicFrame {
                target,
                pos,
                fov,
                roll,
            })
        }
    }
    impl ::core::clone::Clone for CinematicFrame {
        fn clone(&self) -> CinematicFrame {
            *self
        }
    }
    impl ::core::marker::Copy for CinematicFrame {}
    pub struct SoundDetails {
        /// Index into sample_indices
        pub sample_index: u16,
        pub volume: u16,
        pub chance: u16,
        pub details: u16,
    }
    impl tr_readable::Readable for SoundDetails {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let sample_index = tr_readable::Readable::read(reader).unwrap();
            let volume = tr_readable::Readable::read(reader).unwrap();
            let chance = tr_readable::Readable::read(reader).unwrap();
            let details = tr_readable::Readable::read(reader).unwrap();
            Ok(SoundDetails {
                sample_index,
                volume,
                chance,
                details,
            })
        }
    }
    impl ::core::clone::Clone for SoundDetails {
        fn clone(&self) -> SoundDetails {
            *self
        }
    }
    impl ::core::marker::Copy for SoundDetails {}
    pub struct Level {
        pub version: u32,
        pub images: Box<[[u8; NUM_PIXELS]]>,
        pub rooms: Box<[Room]>,
        pub floor_data: Box<[u16]>,
        pub mesh_data: Box<[u16]>,
        pub mesh_offsets: Box<[u32]>,
        pub animations: Box<[Animation]>,
        pub state_changes: Box<[StateChange]>,
        pub anim_dispatches: Box<[AnimDispatch]>,
        pub anim_commands: Box<[u16]>,
        pub mesh_node_data: Box<[u32]>,
        pub frame_data: Box<[u16]>,
        pub models: Box<[Model]>,
        pub static_meshes: Box<[StaticMesh]>,
        pub object_textures: Box<[ObjectTexture]>,
        pub sprite_textures: Box<[SpriteTexture]>,
        pub sprite_sequences: Box<[SpriteSequence]>,
        pub cameras: Box<[Camera]>,
        pub sound_sources: Box<[SoundSource]>,
        pub box_data: BoxData,
        pub animated_textures: Box<[u16]>,
        pub entities: Box<[Entity]>,
        pub light_map: Box<[[u8; PALETTE_SIZE]; LIGHT_MAP_SIZE]>,
        pub palette: Box<[Color3; PALETTE_SIZE]>,
        pub cinematic_frames: Box<[CinematicFrame]>,
        pub demo_data: Box<[u8]>,
        pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
        pub sound_details: Box<[SoundDetails]>,
        pub sample_data: Box<[u8]>,
        pub sample_indices: Box<[u32]>,
    }
    impl tr_readable::Readable for Level {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let version = tr_readable::Readable::read(reader).unwrap();
            let images = tr_readable::read_list_flat::<_, _, u32>(reader).unwrap();
            tr_readable::skip(reader, 4)?;
            let rooms = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let floor_data = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let mesh_data = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let mesh_offsets = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let animations = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let state_changes = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let anim_dispatches = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let anim_commands = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let mesh_node_data = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let frame_data = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let models = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let static_meshes = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let object_textures = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let sprite_textures = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let sprite_sequences = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let cameras = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let sound_sources = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let box_data = tr_readable::Readable::read(reader).unwrap();
            let animated_textures = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let entities = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let light_map = tr_readable::read_boxed_array_flat(reader).unwrap();
            let palette = tr_readable::Readable::read(reader).unwrap();
            let cinematic_frames = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let demo_data = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let sound_map = tr_readable::Readable::read(reader).unwrap();
            let sound_details = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let sample_data = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            let sample_indices = tr_readable::read_list::<_, _, u32>(reader).unwrap();
            Ok(Level {
                version,
                images,
                rooms,
                floor_data,
                mesh_data,
                mesh_offsets,
                animations,
                state_changes,
                anim_dispatches,
                anim_commands,
                mesh_node_data,
                frame_data,
                models,
                static_meshes,
                object_textures,
                sprite_textures,
                sprite_sequences,
                cameras,
                sound_sources,
                box_data,
                animated_textures,
                entities,
                light_map,
                palette,
                cinematic_frames,
                demo_data,
                sound_map,
                sound_details,
                sample_data,
                sample_indices,
            })
        }
    }
    pub enum MeshLighting {
        Normals(Box<[I16Vec3]>),
        Lights(Box<[u16]>),
    }
    impl ::core::clone::Clone for MeshLighting {
        fn clone(&self) -> MeshLighting {
            match self {
                MeshLighting::Normals(__self_0) => {
                    MeshLighting::Normals(::core::clone::Clone::clone(__self_0))
                }
                MeshLighting::Lights(__self_0) => {
                    MeshLighting::Lights(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl Readable for MeshLighting {
        fn read<R: Read>(reader: &mut R) -> Result<Self> {
            Ok(
                match reader.read_i16::<LE>()? {
                    num if num > 0 => {
                        Self::Normals(read_boxed_slice(reader, num as usize)?)
                    }
                    num => Self::Lights(read_boxed_slice(reader, (-num) as usize)?),
                },
            )
        }
    }
    pub struct Mesh {
        pub center: I16Vec3,
        pub radius: i32,
        /// Relative to RoomStaticMesh.pos if static mesh
        pub vertices: Box<[I16Vec3]>,
        pub lighting: MeshLighting,
        pub textured_quads: Box<[TexturedQuad]>,
        pub textured_tris: Box<[TexturedTri]>,
        pub solid_quads: Box<[SolidQuad]>,
        pub solid_tris: Box<[SolidTri]>,
    }
    impl tr_readable::Readable for Mesh {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let center = tr_readable::Readable::read(reader).unwrap();
            let radius = tr_readable::Readable::read(reader).unwrap();
            let vertices = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let lighting = tr_readable::Readable::read(reader).unwrap();
            let textured_quads = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let textured_tris = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let solid_quads = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            let solid_tris = tr_readable::read_list::<_, _, u16>(reader).unwrap();
            Ok(Mesh {
                center,
                radius,
                vertices,
                lighting,
                textured_quads,
                textured_tris,
                solid_quads,
                solid_tris,
            })
        }
    }
    impl ::core::clone::Clone for Mesh {
        fn clone(&self) -> Mesh {
            Mesh {
                center: ::core::clone::Clone::clone(&self.center),
                radius: ::core::clone::Clone::clone(&self.radius),
                vertices: ::core::clone::Clone::clone(&self.vertices),
                lighting: ::core::clone::Clone::clone(&self.lighting),
                textured_quads: ::core::clone::Clone::clone(&self.textured_quads),
                textured_tris: ::core::clone::Clone::clone(&self.textured_tris),
                solid_quads: ::core::clone::Clone::clone(&self.solid_quads),
                solid_tris: ::core::clone::Clone::clone(&self.solid_tris),
            }
        }
    }
    impl<T> ::bitfield::BitRange<T> for MeshNodeFlags
    where
        u32: ::bitfield::BitRange<T>,
    {
        fn bit_range(&self, msb: usize, lsb: usize) -> T {
            self.0.bit_range(msb, lsb)
        }
    }
    impl<T> ::bitfield::BitRangeMut<T> for MeshNodeFlags
    where
        u32: ::bitfield::BitRangeMut<T>,
    {
        fn set_bit_range(&mut self, msb: usize, lsb: usize, value: T) {
            self.0.set_bit_range(msb, lsb, value);
        }
    }
    pub struct MeshNodeFlags(pub u32);
    impl tr_readable::Readable for MeshNodeFlags {
        fn read<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let field0 = tr_readable::Readable::read(reader).unwrap();
            Ok(MeshNodeFlags(field0))
        }
    }
    impl ::core::clone::Clone for MeshNodeFlags {
        fn clone(&self) -> MeshNodeFlags {
            *self
        }
    }
    impl ::core::marker::Copy for MeshNodeFlags {}
    impl MeshNodeFlags {
        pub fn pop(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(0)
        }
        pub fn push(&self) -> bool {
            use ::bitfield::Bit;
            self.bit(1)
        }
    }
    pub struct MeshNode {
        pub flags: MeshNodeFlags,
        /// Relative to parent
        pub offset: IVec3,
    }
    impl ::core::clone::Clone for MeshNode {
        fn clone(&self) -> MeshNode {
            *self
        }
    }
    impl ::core::marker::Copy for MeshNode {}
    pub enum Axis {
        X,
        Y,
        Z,
    }
    impl ::core::clone::Clone for Axis {
        fn clone(&self) -> Axis {
            *self
        }
    }
    impl ::core::marker::Copy for Axis {}
    pub enum FrameRotation {
        Single(Axis, u16),
        /// All values are
        All(U16Vec3),
    }
    impl ::core::clone::Clone for FrameRotation {
        fn clone(&self) -> FrameRotation {
            *self
        }
    }
    impl ::core::marker::Copy for FrameRotation {}
    pub struct Frame {
        pub bound_box: MinMax<I16Vec3>,
        pub offset: I16Vec3,
        pub rotations: Vec<FrameRotation>,
    }
    impl ::core::clone::Clone for Frame {
        fn clone(&self) -> Frame {
            Frame {
                bound_box: ::core::clone::Clone::clone(&self.bound_box),
                offset: ::core::clone::Clone::clone(&self.offset),
                rotations: ::core::clone::Clone::clone(&self.rotations),
            }
        }
    }
    impl Level {
        pub fn get_mesh(&self, mesh_id: u16) -> Mesh {
            let mesh_bytes = unsafe { reinterpret::slice(&self.mesh_data) };
            let mesh_bytes = &mesh_bytes[self.mesh_offsets[mesh_id as usize] as usize..];
            Mesh::read(&mut Cursor::new(mesh_bytes)).expect("parse Mesh from mesh_data")
        }
        /// Should be called with Model.num_meshes - 1
        pub fn get_mesh_nodes(
            &self,
            mesh_node_offset: u32,
            num_meshes: u16,
        ) -> &[MeshNode] {
            let lo = mesh_node_offset as usize;
            let hi = lo + num_meshes as usize * 4;
            unsafe { reinterpret::slice(&self.mesh_node_data[lo..hi]) }
        }
        pub fn get_frame(
            &self,
            single_rot_mask: u16,
            frame_byte_offset: u32,
            num_meshes: u16,
        ) -> Frame {
            let frame_offset = frame_byte_offset as usize / 2;
            let &(bound_box, offset) = unsafe {
                reinterpret::slice_to_ref(&self.frame_data[frame_offset..][..9])
            };
            let mut rotations = Vec::with_capacity(num_meshes as usize);
            let mut frame_offset = frame_offset + 9;
            for _ in 0..num_meshes {
                let word = self.frame_data[frame_offset];
                let (rot, advance) = match word >> 14 {
                    0 => {
                        let word2 = self.frame_data[frame_offset + 1];
                        let rot = U16Vec3 {
                            x: (word >> 4) & 1023,
                            y: ((word & 15) << 6) | (word2 >> 10),
                            z: word2 & 1023,
                        };
                        (FrameRotation::All(rot), 2)
                    }
                    axis => {
                        let axis = match axis {
                            1 => Axis::X,
                            2 => Axis::Y,
                            _ => Axis::Z,
                        };
                        let rot = word & single_rot_mask;
                        (FrameRotation::Single(axis, rot), 1)
                    }
                };
                frame_offset += advance;
                rotations.push(rot);
            }
            Frame {
                bound_box,
                offset,
                rotations,
            }
        }
    }
}
