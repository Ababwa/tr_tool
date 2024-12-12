use crate::vec_tail::VecTail;
use winit::dpi::PhysicalSize;
use std::num::NonZeroU64;
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt, TextureDataOrder}, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType, BufferUsages, CompareFunction, DepthBiasState, DepthStencilState, Device, Extent3d, Queue, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode
};

pub fn buffer(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	device.create_buffer_init(&BufferInitDescriptor { label: None, contents, usage })
}

pub fn writable_uniform(device: &Device, contents: &[u8]) -> Buffer {
	buffer(device, contents, BufferUsages::UNIFORM | BufferUsages::COPY_DST)
}

pub fn shader(device: &Device, source: &str) -> ShaderModule {
	device.create_shader_module(
		ShaderModuleDescriptor { label: None, source: ShaderSource::Wgsl(source.into()) },
	)
}

pub fn buffer_layout_entry(ty: BufferBindingType, size: usize) -> BindingType {
	BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: NonZeroU64::new(size as u64) }
}

pub fn uniform_layout_entry(size: usize) -> BindingType {
	buffer_layout_entry(BufferBindingType::Uniform, size)
}

pub fn storage_layout_entry(size: usize) -> BindingType {
	buffer_layout_entry(BufferBindingType::Storage { read_only: true }, size)
}

pub fn texture_layout_entry(view_dimension: TextureViewDimension) -> BindingType {
	BindingType::Texture { sample_type: TextureSampleType::Uint, view_dimension, multisampled: false }
}

pub fn bind_group_layout(device: &Device, entries: &[(u32, BindingType, ShaderStages)]) -> BindGroupLayout {
	device.create_bind_group_layout(&BindGroupLayoutDescriptor {
		label: None,
		entries: &entries
			.iter()
			.map(|&(binding, ty, visibility)| {
				BindGroupLayoutEntry { binding, visibility, ty, count: None }
			})
			.collect::<Vec<_>>(),
	})
}

pub fn entry(binding: u32, resource: BindingResource) -> BindGroupEntry {
	BindGroupEntry { binding, resource }
}

pub fn bind_group(device: &Device, layout: &BindGroupLayout, entries: &[BindGroupEntry]) -> BindGroup {
	device.create_bind_group(&BindGroupDescriptor { label: None, layout, entries })
}

pub fn texture_desc(
	size: Extent3d, dimension: TextureDimension, format: TextureFormat, usage: TextureUsages,
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

pub fn texture(
	device: &Device, size: Extent3d, dimension: TextureDimension, format: TextureFormat,
	usage: TextureUsages,
) -> Texture {
	device.create_texture(&texture_desc(size, dimension, format, usage))
}

pub fn texture_view_with_data(
	device: &Device, queue: &Queue, size: Extent3d, dimension: TextureDimension, format: TextureFormat,
	usage: TextureUsages, data: &[u8],
) -> TextureView {
	device
		.create_texture_with_data(
			queue, &texture_desc(size, dimension, format, usage), TextureDataOrder::default(), data,
		)
		.create_view(&TextureViewDescriptor::default())
}

pub fn depth_view(device: &Device, PhysicalSize { width, height }: PhysicalSize<u32>) -> TextureView {
	texture(
		device, Extent3d { width, height, depth_or_array_layers: 1 }, TextureDimension::D2,
		TextureFormat::Depth32Float, TextureUsages::RENDER_ATTACHMENT,
	).create_view(&TextureViewDescriptor::default())
}

pub fn vertex_buffer_layouts<'a>(
	attributes_storage: &'a mut Vec<Vec<VertexAttribute>>,
	buffer_params: &[(VertexStepMode, &[VertexFormat])],
) -> Vec<VertexBufferLayout<'a>> {
	*attributes_storage = Vec::with_capacity(buffer_params.len());
	let mut tail = VecTail::new(attributes_storage);
	let mut shader_location = 0;
	let mut buffers = Vec::with_capacity(buffer_params.len());
	for &(step_mode, attribute_formats) in buffer_params {
		let mut attributes = Vec::with_capacity(attribute_formats.len());
		let mut offset = 0;
		for &format in attribute_formats {
			let va = VertexAttribute { format, offset, shader_location };
			offset += format.size();
			shader_location += 1;
			attributes.push(va);
		}
		tail.push(attributes);
		buffers.push(VertexBufferLayout {
			array_stride: offset,
			step_mode,
			attributes: tail.split_one(),
		});
	}
	buffers
}

pub fn depth_stencil_state(depth_write_enabled: bool) -> DepthStencilState {
	DepthStencilState {
		bias: DepthBiasState::default(),
		depth_compare: CompareFunction::Less,
		depth_write_enabled,
		format: TextureFormat::Depth32Float,
		stencil: StencilState::default(),
	}
}
