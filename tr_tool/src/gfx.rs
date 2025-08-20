//! `wgpu` helpers.

use std::{num::NonZero, slice};
use tr_model::tr1;
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt}, wgt::TextureDataOrder, BindGroup, BindGroupDescriptor,
	BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
	BindingType, BlendState, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, Color,
	ColorTargetState, ColorWrites, CommandEncoder, CompareFunction, DepthBiasState, DepthStencilState,
	Device, Extent3d, FragmentState, FrontFace, LoadOp, MultisampleState, Operations,
	PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
	RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
	RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderSource,
	ShaderStages, StencilState, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat,
	TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
	VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;
use crate::as_bytes::AsBytes;

const fn vertex_buffer_layout(
	step_mode: VertexStepMode,
	attribute: &VertexAttribute,
) -> VertexBufferLayout {
	VertexBufferLayout {
		array_stride: attribute.format.size(),
		step_mode,
		attributes: slice::from_ref(attribute),
	}
}

const VERTEX_ATTR: VertexAttribute = VertexAttribute {
	format: VertexFormat::Uint32,
	offset: 0,
	shader_location: 0,
};

const VERTEX_STEP: VertexBufferLayout = vertex_buffer_layout(VertexStepMode::Vertex, &VERTEX_ATTR);

const STORAGE_BUFFER_TYPE: BufferBindingType = BufferBindingType::Storage {
	read_only: true,
};

pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
const INTERACT_TEXTURE_FORMAT: TextureFormat = TextureFormat::R32Uint;

const EGUI_OPS: Operations<Color> = Operations {
	load: LoadOp::Load,
	store: StoreOp::Store,
};
const COLOR_OPS: Operations<Color> = Operations {
	load: LoadOp::Clear(Color::BLACK),
	store: StoreOp::Store,
};
const INTERACT_OPS: Operations<Color> = Operations {
	load: LoadOp::Clear(Color { r: f64::MAX, g: 0.0, b: 0.0, a: 0.0 }),
	store: StoreOp::Store,
};
const DEPTH_OPS: Operations<f32> = Operations {
	load: LoadOp::Clear(1.0),
	store: StoreOp::Store,
};

pub fn shader(device: &Device, source: &str) -> ShaderModule {
	let desc = ShaderModuleDescriptor {
		label: None,
		source: ShaderSource::Wgsl(source.into()),
	};
	device.create_shader_module(desc)
}

const fn buffer_layout_entry(ty: BufferBindingType, size: usize) -> BindingType {
	BindingType::Buffer {
		ty,
		has_dynamic_offset: false,
		min_binding_size: NonZero::new(size as u64),
	}
}

pub const fn uniform_layout_entry(size: usize) -> BindingType {
	buffer_layout_entry(BufferBindingType::Uniform, size)
}

pub const fn storage_layout_entry(size: usize) -> BindingType {
	buffer_layout_entry(STORAGE_BUFFER_TYPE, size)
}

pub const fn texture_layout_entry(view_dimension: TextureViewDimension) -> BindingType {
	BindingType::Texture {
		sample_type: TextureSampleType::Uint,
		view_dimension,
		multisampled: false,
	}
}

pub const fn bind_group_layout_entry(
	binding: u32,
	ty: BindingType,
	visibility: ShaderStages,
) -> BindGroupLayoutEntry {
	BindGroupLayoutEntry {
		binding,
		visibility,
		ty,
		count: None,
	}
}

pub fn bind_group_layout(device: &Device, entries: &[BindGroupLayoutEntry]) -> BindGroupLayout {
	let desc = BindGroupLayoutDescriptor {
		label: None,
		entries,
	};
	device.create_bind_group_layout(&desc)
}

pub fn pipeline(
	device: &Device,
	module: &ShaderModule,
	bind_group_layout: &BindGroupLayout,
	vs_entry: &str,
	fs_entry: &str,
	instance: Option<VertexFormat>,
	cull_mode: Option<wgpu::Face>,
	blend: Option<BlendState>,
	object_data_target: Option<ColorTargetState>,
	depth: bool,
) -> RenderPipeline {
	let pipeline_layout_desc = PipelineLayoutDescriptor {
		label: None,
		bind_group_layouts: &[bind_group_layout],
		push_constant_ranges: &[],
	};
	let layout = device.create_pipeline_layout(&pipeline_layout_desc);
	let mut instance_attr_storage = None;
	let buffers: &[_] = match instance {
		Some(format) => {
			let instance_attr = VertexAttribute {
				format,
				offset: 0,
				shader_location: 1,
			};
			let instance_attr = instance_attr_storage.insert(instance_attr);
			let instance_step = vertex_buffer_layout(VertexStepMode::Instance, instance_attr);
			&[VERTEX_STEP, instance_step]
		},
		None => &[VERTEX_STEP],
	};
	let vertex = VertexState {
		module,
		entry_point: Some(vs_entry),
		compilation_options: PipelineCompilationOptions::default(),
		buffers,
	};
	let primitive = PrimitiveState {
		topology: PrimitiveTopology::TriangleStrip,
		cull_mode,
		front_face: FrontFace::Cw,
		strip_index_format: None,
		..PrimitiveState::default()//other fields require wgpu features
	};
	let depth_stencil = if depth {
		let depth_stencil = DepthStencilState {
			bias: DepthBiasState::default(),
			depth_compare: CompareFunction::Less,
			depth_write_enabled: blend.is_none(),
			format: TextureFormat::Depth32Float,
			stencil: StencilState::default(),
		};
		Some(depth_stencil)
	} else {
		None
	};
	let color_target = ColorTargetState {
		format: TEXTURE_FORMAT,
		blend,
		write_mask: ColorWrites::ALL,
	};
	let targets: &[_] = if object_data_target.is_some() {
		&[Some(color_target), object_data_target]
	} else {
		&[Some(color_target)]
	};
	let fragment = FragmentState {
		module,
		entry_point: Some(fs_entry),
		compilation_options: PipelineCompilationOptions::default(),
		targets,
	};
	let pipeline_desc = RenderPipelineDescriptor {
		label: None,
		layout: Some(&layout),
		vertex,
		primitive,
		depth_stencil,
		multisample: MultisampleState::default(),
		fragment: Some(fragment),
		multiview: None,
		cache: None,
	};
	device.create_render_pipeline(&pipeline_desc)
}

pub fn buffer(device: &Device, size: usize, usage: BufferUsages) -> Buffer {
	let desc = BufferDescriptor {
		label: None,
		size: size as u64,
		usage,
		mapped_at_creation: false,
	};
	device.create_buffer(&desc)
}

pub fn buffer_init(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	let desc = BufferInitDescriptor {
		label: None,
		contents,
		usage,
	};
	device.create_buffer_init(&desc)
}

const fn texture_desc(
	size: Extent3d,
	dimension: TextureDimension,
	format: TextureFormat,
	usage: TextureUsages,
) -> TextureDescriptor<'static> {
	TextureDescriptor {
		label: None,
		size,
		mip_level_count: 1,
		sample_count: 1,
		dimension,
		format,
		usage,
		view_formats: &[],
	}
}

fn texture(
	device: &Device,
	size: Extent3d,
	dimension: TextureDimension,
	format: TextureFormat,
	usage: TextureUsages,
) -> Texture {
	let desc = texture_desc(size, dimension, format, usage);
	device.create_texture(&desc)
}

fn texture_with_data(
	device: &Device,
	queue: &Queue,
	size: Extent3d,
	dimension: TextureDimension,
	format: TextureFormat,
	usage: TextureUsages,
	data: &[u8],
) -> Texture {
	let desc = texture_desc(size, dimension, format, usage);
	device.create_texture_with_data(queue, &desc, TextureDataOrder::default(), data)
}

pub fn interact_texture(device: &Device, size: PhysicalSize<u32>) -> Texture {
	let PhysicalSize { width, height } = size;
	let size = Extent3d {
		width,
		height,
		depth_or_array_layers: 1,
	};
	texture(
		device,
		size,
		TextureDimension::D2,
		INTERACT_TEXTURE_FORMAT,
		TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
	)
}

pub fn palette_view<T>(device: &Device, queue: &Queue, palette: &T) -> TextureView {
	let size = Extent3d {
		width: size_of::<T>() as u32,
		height: 1,
		depth_or_array_layers: 1,
	};
	let texture = texture_with_data(
		device,
		queue,
		size,
		TextureDimension::D1,
		TextureFormat::R8Uint,
		TextureUsages::TEXTURE_BINDING,
		palette.as_bytes(),
	);
	texture.create_view(&TextureViewDescriptor::default())
}

pub fn atlases_view<T>(
	device: &Device,
	queue: &Queue,
	atlases: &[T],
	format: TextureFormat,
) -> TextureView {
	let size = Extent3d {
		width: tr1::ATLAS_SIDE_LEN as u32,
		height: tr1::ATLAS_SIDE_LEN as u32,
		depth_or_array_layers: atlases.len() as u32,
	};
	let texture = texture_with_data(
		device,
		queue,
		size,
		TextureDimension::D2,
		format,
		TextureUsages::TEXTURE_BINDING,
		atlases.as_bytes(),
	);
	texture.create_view(&TextureViewDescriptor::default())
}

pub fn depth_view(device: &Device, size: PhysicalSize<u32>) -> TextureView {
	let PhysicalSize { width, height } = size;
	let size = Extent3d {
		width,
		height,
		depth_or_array_layers: 1,
	};
	let texture = texture(
		device,
		size,
		TextureDimension::D2,
		TextureFormat::Depth32Float,
		TextureUsages::RENDER_ATTACHMENT,
	);
	texture.create_view(&TextureViewDescriptor::default())
}

pub const fn bind_group_entry(binding: u32, resource: BindingResource) -> BindGroupEntry {
	BindGroupEntry {
		binding,
		resource,
	}
}

pub fn bind_group(device: &Device, layout: &BindGroupLayout, entries: &[BindGroupEntry]) -> BindGroup {
	let desc = BindGroupDescriptor {
		label: None,
		layout,
		entries,
	};
	device.create_bind_group(&desc)
}

pub fn egui_render_pass<'a>(encoder: &'a mut CommandEncoder, view: &TextureView) -> RenderPass<'a> {
	let egui_color = RenderPassColorAttachment {
		view: &view,
		resolve_target: None,
		ops: EGUI_OPS,
	};
	let desc = RenderPassDescriptor {
		label: None,
		color_attachments: &[Some(egui_color)],
		depth_stencil_attachment: None,
		timestamp_writes: None,
		occlusion_query_set: None,
	};
	encoder.begin_render_pass(&desc)
}

pub fn main_render_pass<'a>(
	encoder: &'a mut CommandEncoder,
	color_view: &TextureView,
	interact_view: &TextureView,
	depth_view: &TextureView,
) -> RenderPass<'a> {
	let color_attachment = RenderPassColorAttachment {
		view: color_view,
		resolve_target: None,
		ops: COLOR_OPS,
	};
	let interact_attachment = RenderPassColorAttachment {
		view: interact_view,
		resolve_target: None,
		ops: INTERACT_OPS,
	};
	let depth_attachment = RenderPassDepthStencilAttachment {
		view: depth_view,
		depth_ops: Some(DEPTH_OPS),
		stencil_ops: None,
	};
	let desc = RenderPassDescriptor {
		label: None,
		color_attachments: &[Some(color_attachment), Some(interact_attachment)],
		depth_stencil_attachment: Some(depth_attachment),
		timestamp_writes: None,
		occlusion_query_set: None,
	};
	encoder.begin_render_pass(&desc)
}
