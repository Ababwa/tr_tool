use std::{borrow::Cow, f32::consts::FRAC_PI_4, mem::size_of, num::NonZeroU64};
use glam::{vec3, EulerRot, Mat4, Vec3, Vec3Swizzles};
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt, TextureDataOrder},
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
	BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferBindingType,
	BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
	DepthStencilState, Device, Extent3d, Face, FragmentState, MultisampleState,
	PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPipeline, RenderPipelineDescriptor,
	ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState,
	TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
	TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout,
	VertexFormat, VertexState, VertexStepMode,
};
use winit::dpi::PhysicalSize;

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub fn perspective_transform(window_size: PhysicalSize<u32>) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.width as f32 / window_size.height as f32, 0.1, 200.0)
}

pub fn camera_transform(cam_pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	Mat4::from_euler(EulerRot::XYZ, pitch, yaw, 0.0) * Mat4::from_translation(cam_pos) * Mat4::from_scale(vec3(1.0, -1.0, -1.0))
}

pub fn yaw_pitch(v: Vec3) -> (f32, f32) {
	(v.z.atan2(v.x), v.y.atan2(v.xz().length()))
}

pub fn depth_view(device: &Device, window_size: PhysicalSize<u32>) -> TextureView {
	device.create_texture(&TextureDescriptor {
		label: None,
		size: Extent3d {
			width: window_size.width,
			height: window_size.height,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: TextureDimension::D2,
		format: DEPTH_FORMAT,
		usage: TextureUsages::RENDER_ATTACHMENT,
		view_formats: &[],
	}).create_view(&TextureViewDescriptor::default())
}

fn buffer(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	device.create_buffer_init(&BufferInitDescriptor { label: None, contents, usage })
}

pub fn uniform_buffer(device: &Device, contents: &[u8]) -> Buffer {
	buffer(device, contents, BufferUsages::UNIFORM | BufferUsages::COPY_DST)
}

pub fn vertex_buffer(device: &Device, contents: &[u8]) -> Buffer {
	buffer(device, contents, BufferUsages::VERTEX)
}

const MATRIX_BIND_GROUP_LAYOUT_ENTRY: BindGroupLayoutEntry = BindGroupLayoutEntry {
	binding: 0,
	visibility: ShaderStages::VERTEX,
	count: None,
	ty: BindingType::Buffer {
		ty: BufferBindingType::Uniform,
		has_dynamic_offset: false,
		min_binding_size: NonZeroU64::new(size_of::<Mat4>() as u64),
	},
};

pub fn bind_group_layout(device: &Device, view_dimension: TextureViewDimension) -> BindGroupLayout {
	device.create_bind_group_layout(&BindGroupLayoutDescriptor {
		label: None,
		entries: &[
			BindGroupLayoutEntry { binding: 0, ..MATRIX_BIND_GROUP_LAYOUT_ENTRY },
			BindGroupLayoutEntry { binding: 1, ..MATRIX_BIND_GROUP_LAYOUT_ENTRY },
			BindGroupLayoutEntry {
				binding: 2,
				visibility: ShaderStages::FRAGMENT,
				count: None,
				ty: BindingType::Texture {
					sample_type: TextureSampleType::Float { filterable: false },
					view_dimension,
					multisampled: false,
				},
			},
		],
	})
}

pub fn bind_group(
	device: &Device,
	queue: &Queue,
	layout: &BindGroupLayout,
	perspective_buffer: &Buffer,
	camera_buffer: &Buffer,
	size: Extent3d,
	dimension: TextureDimension,
	format: TextureFormat,
	data: &[u8],
) -> BindGroup {
	device.create_bind_group(&BindGroupDescriptor {
		label: None,
		layout,
		entries: &[
			BindGroupEntry { binding: 0, resource: perspective_buffer.as_entire_binding() },
			BindGroupEntry { binding: 1, resource: camera_buffer.as_entire_binding() },
			BindGroupEntry {
				binding: 2,
				resource: BindingResource::TextureView(&device.create_texture_with_data(
					queue,
					&TextureDescriptor {
						label: None,
						size,
						mip_level_count: 1,
						sample_count: 1,
						dimension,
						format,
						usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
						view_formats: &[],
					},
					TextureDataOrder::default(),
					data,
				).create_view(&TextureViewDescriptor::default()))
			}
		],
	})
}

pub fn shader(device: &Device, contents: &str) -> ShaderModule {
	device.create_shader_module(ShaderModuleDescriptor {
		label: None,
		source: ShaderSource::Wgsl(Cow::Borrowed(contents)),
	})
}

pub fn vertex_attributes(formats: &[VertexFormat]) -> Vec<VertexAttribute> {
	let mut offset = 0;
	formats.iter().enumerate().map(|(index, &format)| {
		let va = VertexAttribute {
			format,
			offset,
			shader_location: index as u32,
		};
		offset += format.size();
		va
	}).collect()
}

pub fn vertex_buffer_layout(array_stride: u64, attributes: &[VertexAttribute]) -> VertexBufferLayout {
	VertexBufferLayout {
		array_stride,
		step_mode: VertexStepMode::Vertex,
		attributes,
	}
}

pub fn vertex_state<'a>(module: &'a ShaderModule, buffers: &'a [VertexBufferLayout]) -> VertexState<'a> {
	VertexState {
		module,
		entry_point: "vs_main",
		buffers,
	}
}

pub fn pipeline_layout_descriptor<'a>(bind_group_layouts: &'a [&BindGroupLayout]) -> PipelineLayoutDescriptor<'a> {
	PipelineLayoutDescriptor {
		label: None,
		bind_group_layouts,
		push_constant_ranges: &[],
	}
}

pub fn render_pipeline(
	device: &Device,
	pipeline_layout_descriptor: &PipelineLayoutDescriptor,
	vertex: VertexState,
	module: &ShaderModule,
	blend: Option<BlendState>,
	depth_write_enabled: bool,
) -> RenderPipeline {
	device.create_render_pipeline(&RenderPipelineDescriptor {
		label: None,
		layout: Some(&device.create_pipeline_layout(pipeline_layout_descriptor)),
		vertex,
		fragment: Some(FragmentState {
			module,
			entry_point: "fs_main",
			targets: &[Some(ColorTargetState {
				blend,
				format: TextureFormat::Bgra8UnormSrgb,
				write_mask: ColorWrites::all(),
			})],
		}),
		primitive: PrimitiveState { cull_mode: Some(Face::Front), ..PrimitiveState::default() },
		depth_stencil: Some(DepthStencilState {
			bias: DepthBiasState::default(),
			depth_compare: CompareFunction::Less,
			depth_write_enabled,
			format: DEPTH_FORMAT,
			stencil: StencilState::default(),
		}),
		multisample: MultisampleState::default(),
		multiview: None,
	})
}
