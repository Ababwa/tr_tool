use std::mem::transmute;
use glam::Mat4;
use wgpu::{
	BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingResource, BlendComponent, BlendFactor,
	BlendOperation, BlendState, Buffer, BufferUsages, ColorTargetState, ColorWrites, Device, RenderPipeline,
	ShaderModule, ShaderStages, TextureFormat, TextureViewDimension, VertexFormat,
};
use crate::{as_bytes::AsBytes, gfx::{self, bind_group_layout_entry as entry}, GEOM_BUFFER_SIZE};

#[derive(Clone)]
pub struct PipelineGroup {
	pub opaque_pl: RenderPipeline,
	pub additive_pl: RenderPipeline,
	pub sprite_pl: RenderPipeline,
	pub flat_pl: RenderPipeline,
}

pub struct BindingBuffers {
	pub geom_buffer: Buffer,
	pub geom_offsets_buffer: Buffer,
	pub camera_transform_buffer: Buffer,
	pub perspective_transform_buffer: Buffer,
	pub scroll_offset_buffer: Buffer,
	pub viewport_buffer: Buffer,
}

pub struct BufferEntries<'a>([BindGroupEntry<'a>; 8]);

pub struct BindGroupLayouts {
	pub palette_bgl: BindGroupLayout,
	pub texture_bgl: BindGroupLayout,
	pub solid_32bit_bgl: BindGroupLayout,
}

pub struct RenderResources {
	pub bind_group_layouts: BindGroupLayouts,
	pub binding_buffers: BindingBuffers,
	pub face_vertex_indices_buffer: Buffer,
	pub reverse_indices_buffer: Buffer,
	pub texture_palette_plg: PipelineGroup,
	pub texture_16bit_plg: PipelineGroup,
	pub texture_32bit_plg: PipelineGroup,
	pub solid_24bit_pl: RenderPipeline,
	pub solid_32bit_pl: RenderPipeline,
}

#[repr(C)]
pub struct GeomOffsets {
	pub transforms_offset: u32,
	pub face_array_offsets_offset: u32,
	pub object_textures_offset: u32,
	pub sprite_textures_offset: u32,
	pub object_texture_size: u32,
	pub num_atlases: u32,
}

#[repr(C)]
pub struct Viewport {
	pub clip: [i32; 4],
	pub view: [i32; 4],
}

struct PipelineArgs {
	instance: Option<VertexFormat>,
	cull_mode: Option<wgpu::Face>,
	blend: Option<BlendState>,
	object_data_target: Option<ColorTargetState>,
	depth: bool,
}

struct PipelineMaker<'a> {
	device: &'a Device,
	shader: &'a ShaderModule,
}

const GEOM_ENTRY: u32 = 0;
const GEOM_OFFSETS_ENTRY: u32 = 1;
const CAMERA_ENTRY: u32 = 2;
const PERSPECTIVE_ENTRY: u32 = 3;
pub const PALETTE_ENTRY: u32 = 4;
pub const ATLASES_ENTRY: u32 = 5;
const VIEWPORT_ENTRY: u32 = 6;
const SCROLL_OFFSET_ENTRY: u32 = 7;

/// Palette at front and atlases at back to make omittable for texture and solid pipelines.
const ENTRIES: [BindGroupLayoutEntry; 8] = [
	entry(PALETTE_ENTRY, gfx::texture_layout_entry(TextureViewDimension::D1), ShaderStages::FRAGMENT),
	entry(GEOM_ENTRY, gfx::storage_layout_entry(GEOM_BUFFER_SIZE), ShaderStages::VERTEX),
	entry(GEOM_OFFSETS_ENTRY, gfx::uniform_layout_entry(size_of::<GeomOffsets>()), ShaderStages::VERTEX),
	entry(CAMERA_ENTRY, gfx::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),
	entry(PERSPECTIVE_ENTRY, gfx::uniform_layout_entry(size_of::<Mat4>()), ShaderStages::VERTEX),
	entry(VIEWPORT_ENTRY, gfx::uniform_layout_entry(size_of::<Viewport>()), ShaderStages::VERTEX),
	entry(SCROLL_OFFSET_ENTRY, gfx::uniform_layout_entry(size_of::<egui::Vec2>()), ShaderStages::VERTEX),
	entry(ATLASES_ENTRY, gfx::texture_layout_entry(TextureViewDimension::D2Array), ShaderStages::FRAGMENT),
];

const OBJECT_DATA_TARGET: ColorTargetState = ColorTargetState {
	format: TextureFormat::R32Uint,
	blend: None,
	write_mask: ColorWrites::ALL,
};

const ADDITIVE_BLEND: BlendState = BlendState {
	alpha: BlendComponent {
		src_factor: BlendFactor::One,
		dst_factor: BlendFactor::One,
		operation: BlendOperation::Add,
	},
	color: BlendComponent {
		src_factor: BlendFactor::One,
		dst_factor: BlendFactor::One,
		operation: BlendOperation::Add,
	},
};

const OPAQUE_ARGS: PipelineArgs = PipelineArgs {
	instance: Some(VertexFormat::Uint32x3),
	cull_mode: Some(wgpu::Face::Back),
	blend: None,
	object_data_target: Some(OBJECT_DATA_TARGET),
	depth: true,
};
const ADDITIVE_ARGS: PipelineArgs = PipelineArgs {
	instance: Some(VertexFormat::Uint32x3),
	cull_mode: Some(wgpu::Face::Back),
	blend: Some(ADDITIVE_BLEND),
	object_data_target: Some(OBJECT_DATA_TARGET),
	depth: true,
};
const SPRITE_ARGS: PipelineArgs = PipelineArgs {
	instance: Some(VertexFormat::Sint32x4),
	cull_mode: Some(wgpu::Face::Back),
	blend: None,
	object_data_target: Some(OBJECT_DATA_TARGET),
	depth: true,
};
const FLAT_ARGS: PipelineArgs = PipelineArgs {
	instance: None,
	cull_mode: None,
	blend: None,
	object_data_target: None,
	depth: false,
};

/**
This ordering creates a "Z" so triangle strip mode may be used for quads, and the first three indices used
for tris.
*/
const FACE_VERTEX_INDICES: [u32; 4] = [1, 2, 0, 3];

/// Yields face vertex indices `[1, 0, 2, 3]`.
const REVERSE_INDICES: [u16; 4] = [0, 2, 1, 3];

const VERTEX: BufferUsages = BufferUsages::VERTEX;
const INDEX: BufferUsages = BufferUsages::INDEX;

const UNIFORM: BufferUsages = BufferUsages::union(BufferUsages::UNIFORM, BufferUsages::COPY_DST);
const STORAGE: BufferUsages = BufferUsages::union(BufferUsages::STORAGE, BufferUsages::COPY_DST);

impl BindingBuffers {
	pub fn entries(&self) -> BufferEntries {
		let entries = [
			gfx::bind_group_entry(GEOM_ENTRY, self.geom_buffer.as_entire_binding()),
			gfx::bind_group_entry(GEOM_OFFSETS_ENTRY, self.geom_offsets_buffer.as_entire_binding()),
			gfx::bind_group_entry(CAMERA_ENTRY, self.camera_transform_buffer.as_entire_binding()),
			gfx::bind_group_entry(PERSPECTIVE_ENTRY, self.perspective_transform_buffer.as_entire_binding()),
			gfx::bind_group_entry(VIEWPORT_ENTRY, self.viewport_buffer.as_entire_binding()),
			gfx::bind_group_entry(SCROLL_OFFSET_ENTRY, self.scroll_offset_buffer.as_entire_binding()),
			gfx::bind_group_entry(u32::MAX, BindingResource::BufferArray(&[])),//dummies will be overwritten
			gfx::bind_group_entry(u32::MAX, BindingResource::BufferArray(&[])),
		];
		BufferEntries(entries)
	}
}

impl<'a> BufferEntries<'a> {
	pub fn with<'b>(&'b mut self, entry: BindGroupEntry<'b>) -> &'b [BindGroupEntry<'b>; 7] {
		//Safety: 'a outlives 'b. `entry` never exposed as 'a. `[_; 7]` sub array of `[_; 8]`.
		let entries = unsafe {
			transmute::<_, &mut [BindGroupEntry<'b>; 7]>(&mut self.0)
		};
		entries[6] = entry;
		entries
	}
	
	pub fn with_both<'b>(
		&'b mut self,
		entry1: BindGroupEntry<'b>,
		entry2: BindGroupEntry<'b>,
	) -> &'b [BindGroupEntry<'b>; 8] {
		//Safety: See `with`.
		let entries = unsafe {
			transmute::<_, &mut [BindGroupEntry<'b>; 8]>(&mut self.0)
		};
		entries[6] = entry1;
		entries[7] = entry2;
		entries
	}
}

impl<'a> PipelineMaker<'a> {
	fn make(
		&self,
		vs_entry: &str,
		fs_entry: &str,
		bind_group_layout: &BindGroupLayout,
		args: PipelineArgs,
	) -> RenderPipeline {
		gfx::pipeline(
			self.device,
			self.shader,
			bind_group_layout,
			vs_entry,
			fs_entry,
			args.instance,
			args.cull_mode,
			args.blend,
			args.object_data_target,
			args.depth,
		)
	}
	
	fn group(
		&self,
		texture_fs_entry: &str,
		flat_fs_entry: &str,
		bind_group_layout: &BindGroupLayout,
	) -> PipelineGroup {
		PipelineGroup {
			opaque_pl: self.make("texture_vs_main", texture_fs_entry, bind_group_layout, OPAQUE_ARGS),
			additive_pl: self.make("texture_vs_main", texture_fs_entry, bind_group_layout, ADDITIVE_ARGS),
			sprite_pl: self.make("sprite_vs_main", texture_fs_entry, bind_group_layout, SPRITE_ARGS),
			flat_pl: self.make("flat_vs_main", flat_fs_entry, bind_group_layout, FLAT_ARGS),
		}
	}
}

impl RenderResources {
	pub fn new(device: &Device) -> Self {
		let shader = gfx::shader(device, include_str!("shader/mesh.wgsl"));
		let palette_bgl = gfx::bind_group_layout(device, &ENTRIES);
		let texture_bgl = gfx::bind_group_layout(device, &ENTRIES[1..]);
		let solid_32bit_bgl = gfx::bind_group_layout(device, &ENTRIES[..7]);
		let pm = PipelineMaker {
			device,
			shader: &shader,
		};
		let texture_palette_plg = pm.group("texture_palette_fs_main", "flat_palette_fs_main", &palette_bgl);
		let texture_16bit_plg = pm.group("texture_16bit_fs_main", "flat_16bit_fs_main", &texture_bgl);
		let texture_32bit_plg = pm.group("texture_32bit_fs_main", "flat_32bit_fs_main", &texture_bgl);
		let solid_24bit_pl = pm.make("solid_24bit_vs_main", "solid_24bit_fs_main", &palette_bgl, OPAQUE_ARGS);
		let solid_32bit_pl = pm.make("solid_32bit_vs_main", "solid_32bit_fs_main", &solid_32bit_bgl, OPAQUE_ARGS);
		let face_vertex_indices_buffer = gfx::buffer_init(device, FACE_VERTEX_INDICES.as_bytes(), VERTEX);
		let reverse_indices_buffer = gfx::buffer_init(device, REVERSE_INDICES.as_bytes(), INDEX);
		let geom_buffer = gfx::buffer(device, GEOM_BUFFER_SIZE, STORAGE);
		let geom_offsets_buffer = gfx::buffer(device, size_of::<GeomOffsets>(), UNIFORM);
		let camera_transform_buffer = gfx::buffer(device, size_of::<Mat4>(), UNIFORM);
		let perspective_transform_buffer = gfx::buffer(device, size_of::<Mat4>(), UNIFORM);
		let scroll_offset_buffer = gfx::buffer(device, size_of::<egui::Vec2>(), UNIFORM);
		let viewport_buffer = gfx::buffer(device, size_of::<Viewport>(), UNIFORM);
		let bind_group_layouts = BindGroupLayouts {
			palette_bgl,
			texture_bgl,
			solid_32bit_bgl,
		};
		let binding_buffers = BindingBuffers {
			geom_buffer,
			geom_offsets_buffer,
			camera_transform_buffer,
			perspective_transform_buffer,
			scroll_offset_buffer,
			viewport_buffer,
		};
		RenderResources {
			bind_group_layouts,
			binding_buffers,
			face_vertex_indices_buffer,
			reverse_indices_buffer,
			texture_palette_plg,
			texture_16bit_plg,
			texture_32bit_plg,
			solid_24bit_pl,
			solid_32bit_pl,
		}
	}
}
