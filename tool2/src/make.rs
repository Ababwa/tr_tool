use crate::vec_tail::VecTail;
use glam::UVec2;
use std::num::NonZeroU64;
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt}, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
	BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
	BufferUsages, Device, Extent3d, ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages,
	Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
	TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
};

pub fn buffer(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	device.create_buffer_init(&BufferInitDescriptor { label: None, contents, usage })
}

pub fn shader(device: &Device, source: &str) -> ShaderModule {
	device.create_shader_module(ShaderModuleDescriptor {
		label: None,
		source: ShaderSource::Wgsl(source.into()),
	})
}

pub fn layout_entry(ty: BufferBindingType, size: usize) -> BindingType {
	BindingType::Buffer {
		ty,
		has_dynamic_offset: false,
		min_binding_size: NonZeroU64::new(size as u64),
	}
}

pub fn uniform_layout_entry(size: usize) -> BindingType {
	layout_entry(BufferBindingType::Uniform, size)
}

pub fn storage_layout_entry(size: usize) -> BindingType {
	layout_entry(BufferBindingType::Storage { read_only: true }, size)
}

pub fn bind_group_layout(device: &Device, entries: &[(BindingType, ShaderStages)]) -> BindGroupLayout {
	device.create_bind_group_layout(&BindGroupLayoutDescriptor {
		label: None,
		entries: &entries
			.iter()
			.enumerate()
			.map(|(index, &(ty, visibility))| {
				BindGroupLayoutEntry {
					binding: index as u32,
					visibility,
					ty,
					count: None,
				}
			})
			.collect::<Vec<_>>(),
	})
}

pub fn bind_group(device: &Device, layout: &BindGroupLayout, entries: &[BindingResource]) -> BindGroup {
	device.create_bind_group(&BindGroupDescriptor {
		label: None,
		layout,
		entries: &entries
			.iter()
			.cloned()
			.enumerate()
			.map(|(index, resource)| BindGroupEntry { binding: index as u32, resource })
			.collect::<Vec<_>>(),
	})
}

pub fn texture(device: &Device, size: UVec2, format: TextureFormat, usage: TextureUsages) -> Texture {
	device.create_texture(&TextureDescriptor {
		label: None,
		size: Extent3d {
			width: size.x,
			height: size.y,
			depth_or_array_layers: 1,
		},
		mip_level_count: 1,
		sample_count: 1,
		dimension: TextureDimension::D2,
		format,
		usage,
		view_formats: &[],
	})
}

pub fn depth_view(device: &Device, size: UVec2) -> TextureView {
	texture(device, size, TextureFormat::Depth32Float, TextureUsages::RENDER_ATTACHMENT)
		.create_view(&TextureViewDescriptor::default())
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