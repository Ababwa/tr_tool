use std::{f32::consts::FRAC_PI_4, mem::size_of, num::NonZeroU64};
use glam::{EulerRot, Mat4, UVec2, Vec3};
use wgpu::{
	util::{BufferInitDescriptor, DeviceExt},
	BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
	BindingResource, BindingType, Buffer, BufferBindingType, BufferUsages, Device, Extent3d, ShaderModule,
	ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat,
	TextureUsages, TextureView, TextureViewDescriptor,
};

pub fn buffer(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	device.create_buffer_init(
		&BufferInitDescriptor {
			label: None,
			contents,
			usage,
		},
	)
}

pub fn buffer_fixed<T>(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
	let mut fixed_buffer = vec![0u8; size_of::<T>()];
	fixed_buffer[..contents.len()].copy_from_slice(contents);
	buffer(device, &fixed_buffer, usage)
}

pub fn shader(device: &Device, shader_source: &str) -> ShaderModule {
	device.create_shader_module(
		ShaderModuleDescriptor {
			label: None,
			source: ShaderSource::Wgsl(shader_source.into()),
		},
	)
}

pub fn layout_entry<T>(ty: BufferBindingType) -> BindingType {
	BindingType::Buffer {
		ty,
		has_dynamic_offset: false,
		min_binding_size: NonZeroU64::new(size_of::<T>() as u64),
	}
}

pub fn uniform_layout_entry<T>() -> BindingType {
	layout_entry::<T>(BufferBindingType::Uniform)
}

// pub fn storage_layout_entry<T>() -> BindingType {
// 	layout_entry::<T>(BufferBindingType::Storage { read_only: true })
// }

pub fn bind_group_layout(device: &Device, entries: &[(BindingType, ShaderStages)]) -> BindGroupLayout {
	device.create_bind_group_layout(
		&BindGroupLayoutDescriptor {
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
		},
	)
}

pub fn bind_group(device: &Device, layout: &BindGroupLayout, entries: &[BindingResource]) -> BindGroup {
	device.create_bind_group(
		&BindGroupDescriptor {
			label: None,
			layout,
			entries: &entries
				.iter()
				.cloned()
				.enumerate()
				.map(|(index, resource)| {
					BindGroupEntry {
						binding: index as u32,
						resource,
					}
				})
				.collect::<Vec<_>>(),
		},
	)
}

pub fn depth_view(device: &Device, window_size: UVec2) -> TextureView {
	device
		.create_texture(
			&TextureDescriptor {
				label: None,
				size: Extent3d {
					width: window_size.x,
					height: window_size.y,
					depth_or_array_layers: 1,
				},
				mip_level_count: 1,
				sample_count: 1,
				dimension: TextureDimension::D2,
				format: TextureFormat::Depth32Float,
				usage: TextureUsages::RENDER_ATTACHMENT,
				view_formats: &[],
			},
		)
		.create_view(&TextureViewDescriptor::default())
}

pub fn transform(window_size: UVec2, pos: Vec3, yaw: f32, pitch: f32) -> Mat4 {
	Mat4::perspective_rh(FRAC_PI_4, window_size.x as f32 / window_size.y as f32, 0.1, 200000.0) *
	Mat4::from_euler(EulerRot::XYZ, pitch, yaw, 0.0) *
	Mat4::from_translation(pos)
	// * Mat4::from_scale(vec3(1.0, -1.0, -1.0))
}
