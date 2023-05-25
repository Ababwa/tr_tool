use std::{fs::File, io::BufReader, env::args};
use anyhow::{Result, anyhow};
use bevy::{prelude::*, DefaultPlugins, window::{WindowPlugin, CursorGrabMode, WindowResolution}, core_pipeline::clear_color::ClearColorConfig, math::{vec3, Vec3Swizzles, vec2}, render::{render_resource::{PrimitiveTopology, TextureFormat, Extent3d, TextureDimension, AsBindGroup, ShaderRef}, mesh::Indices, texture::ImageSampler}, time::Time, app::AppExit, reflect::TypeUuid};
use leafwing_input_manager::{Actionlike, prelude::{ActionState, InputMap, DualAxis, InputManagerPlugin}};
use smooth_bevy_cameras::{LookTransformBundle, LookTransform, Smoother, LookAngles, LookTransformPlugin};
use tr_reader::{Readable, tr4_model as tr4};
use tr_tool::Rotatable;

const IMG_DIM: u32 = tr4::IMG_DIM as u32;
const WINDOW_WIDTH: f32 = 1920.0;
const WINDOW_HEIGHT: f32 = 1080.0;

#[derive(Actionlike, Clone)]
enum CameraAction {
	Look,
	Toggle,
	Forward,
	Backward,
	Left,
	Right,
	Up,
	Down,
	Shift,
}

#[derive(Resource)]
struct CameraControl(bool);

#[derive(Component)]
struct Scene;

#[derive(TypeUuid, AsBindGroup, Clone)]
#[uuid = "310f88a1-f66a-4bed-8960-18554ec3da95"]
struct FaceMaterial {
	#[texture(0)]
	#[sampler(1)]
	texture: Handle<Image>,
}

impl Material for FaceMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/face.wgsl".into()
	}
}

fn get_vertex(room: &tr4::Room, vertex: &tr4::Vertex<i16>) -> Vec3 {
	vec3((room.x + vertex.x as i32) as f32, vertex.y as f32, (room.z + vertex.z as i32) as f32) / 1024.0
}

const TEXTURE_ID_MASK: u16 = 32767;
const ATLAS_ID_MASK: u16 = 32767;
//const TEXTURE_DOUBLE_SIDED_MASK: u16 = 32768;

//normalize TR texture coord
//units are in 256ths of a pixel, images are 256 pixels wide, divide by 256*256 to normalize
fn ntc(c: u16) -> f32 {
	c as f32 / 65536.0
}

struct Bounds {
	min: Vec3,
	max: Vec3,
}

impl Bounds {
	fn new() -> Self {
		Bounds { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) }
	}
	
	fn update(&mut self, v: Vec3) {
		self.min = self.min.min(v);
		self.max = self.max.max(v);
	}
}

fn add_mesh<const N: usize>(
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<FaceMaterial>>,
	images: &[Handle<Image>],
	object_textures: &[tr4::ObjectTexture],
	room: &tr4::Room,
	face: &tr4::RoomFace<N>,
	indices: &[u16],
	bounds: &mut Bounds,
) {
	let verts: Vec<_> = face.vertex_ids.iter().map(|&id| get_vertex(room, &room.vertices[id as usize].vertex)).collect();
	for v in &verts {
		bounds.update(*v);
	}
	let object_texture = &object_textures[(face.texture_and_flag & TEXTURE_ID_MASK) as usize];
	let tex_verts: Vec<_> = object_texture.vertices[..N].iter().map(|v| vec2(ntc(v.x), ntc(v.y))).collect();
	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_verts);
	mesh.set_indices(Some(Indices::U16(indices.to_vec())));
	commands.spawn(MaterialMeshBundle {
		mesh: meshes.add(mesh),
		material: materials.add(FaceMaterial { texture: images[(object_texture.atlas_and_flag & ATLAS_ID_MASK) as usize].clone() }),
		..Default::default()
	}).insert(Scene);
}

fn build_scene(
	commands: &mut Commands,
	meshes: &mut ResMut<Assets<Mesh>>,
	materials: &mut ResMut<Assets<FaceMaterial>>,
	images: &[Handle<Image>],
	level_data: &tr4::LevelData,
	old_scene: Option<Query<Entity, With<Scene>>>,
) -> Bounds {
	if let Some(old_scene) = old_scene {
		for entity in &old_scene {
			commands.entity(entity).despawn_recursive();
		}
	}
	let quad_indices = [2, 1, 0, 0, 3, 2];
	let triangle_indices = [2, 1, 0];
	let mut bounds = Bounds::new();
	for room in &level_data.rooms {
		for quad in &room.quads {
			add_mesh(commands, meshes, materials, images, &level_data.object_textures, room, quad, &quad_indices, &mut bounds);
		}
		for triangle in &room.triangles {
			add_mesh(commands, meshes, materials, images, &level_data.object_textures, room, triangle, &triangle_indices, &mut bounds);
		}
	}
	bounds
}

const CAM_ROT_DIVISOR: f32 = 200.0;
const CAM_SPEED_SLOW: f32 = 10.0;
const CAM_SPEED_FAST: f32 = 30.0;

fn camera_control(
	mut look_transform: Query<&mut LookTransform>,
	mut windows: Query<&mut Window>,
	mut camera_control: ResMut<CameraControl>,
	action_state: Res<ActionState<CameraAction>>,
	time: Res<Time>,
) {
	if action_state.just_pressed(CameraAction::Toggle) {
		let mut window = windows.single_mut();
		window.cursor.visible = camera_control.0;
		window.cursor.grab_mode = if camera_control.0 { CursorGrabMode::None } else { CursorGrabMode::Confined };
		camera_control.0 ^= true;
	}
	if camera_control.0 {
		let mut look_transform = look_transform.single_mut();
		let mut angles = LookAngles::from_vector(look_transform.look_direction().unwrap());
		let axes = action_state.axis_pair(CameraAction::Look).unwrap();
		angles.add_yaw(axes.x() / CAM_ROT_DIVISOR);
		angles.add_pitch(axes.y() / CAM_ROT_DIVISOR);
		look_transform.target = look_transform.eye + angles.unit_vector();
		let forward = look_transform.look_direction().unwrap().xz().extend(0.0).xzy().normalize();
		let left = forward.rotate_xz();
		let speed = if action_state.pressed(CameraAction::Shift) { CAM_SPEED_FAST } else { CAM_SPEED_SLOW };
		let delta = time.delta_seconds() * speed * [
			(CameraAction::Forward, forward),
			(CameraAction::Backward, -forward),
			(CameraAction::Left, left),
			(CameraAction::Right, -left),
			(CameraAction::Up, Vec3::NEG_Y),
			(CameraAction::Down, Vec3::Y),
		].into_iter().filter_map(|(action, dir)| action_state.pressed(action).then_some(dir)).fold(Vec3::ZERO, |a, b| a + b);
		look_transform.eye += delta;
		look_transform.target += delta;
	}
}

#[derive(Resource)]
struct LevelPath(String);

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<FaceMaterial>>,
	mut images: ResMut<Assets<Image>>,
	level_path: Res<LevelPath>,
) {
	let tr4::Level { images_32, level_data, .. } = tr4::Level::read(&mut BufReader::new(File::open(&level_path.0).expect("failed to open file"))).expect("failed to read level");
	let images: Vec<_> = images_32.into_iter().map(|img| {
		let mut image = Image::new(
			Extent3d { width: IMG_DIM, height: IMG_DIM, depth_or_array_layers: 1 },
			TextureDimension::D2,
			(img as Box<[_]>).into_vec(),
			TextureFormat::Bgra8UnormSrgb,
		);
		image.sampler_descriptor = ImageSampler::nearest();
		images.add(image)
	}).collect();
	let Bounds { min, max } = build_scene(&mut commands, &mut meshes, &mut materials, &images, &level_data, None);
	commands.spawn(Camera3dBundle {
		camera_3d: Camera3d {
			clear_color: ClearColorConfig::Custom(Color::rgb(0.1, 0.3, 0.2)),
			..Default::default()
		},
		..Default::default()
	}).insert(LookTransformBundle {
		transform: LookTransform::new(min, max, Vec3::NEG_Y),
		smoother: Smoother::new(0.8),
	});
}

fn escape_quit(keyboard: Res<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
	if keyboard.pressed(KeyCode::Escape) {
		exit.send(AppExit);
	}
}

fn main() -> Result<()> {
	let path = match args().skip(1).next() {
		Some(path) => path,
		None => return Err(anyhow!("Path to .tr4 file must be provided")),
	};
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "TR Reader".to_owned(),
				resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
				..Default::default()
			}),
			..Default::default()
		}))
		.add_plugin(LookTransformPlugin)
		.add_plugin(InputManagerPlugin::<CameraAction>::default())
		.add_plugin(MaterialPlugin::<FaceMaterial>::default())
		.insert_resource(CameraControl(false))
		.insert_resource(ActionState::<CameraAction>::default())
		.insert_resource(InputMap::<CameraAction>::default()
			.insert(DualAxis::mouse_motion(), CameraAction::Look)
			.insert(MouseButton::Right, CameraAction::Toggle)
			.insert(KeyCode::W, CameraAction::Forward)
			.insert(KeyCode::A, CameraAction::Left)
			.insert(KeyCode::S, CameraAction::Backward)
			.insert(KeyCode::D, CameraAction::Right)
			.insert(KeyCode::Q, CameraAction::Up)
			.insert(KeyCode::E, CameraAction::Down)
			.insert(KeyCode::LShift, CameraAction::Shift)
			.build())
		.insert_resource(LevelPath(path))
		.add_startup_system(setup)
		.add_system(camera_control)
		.add_system(escape_quit)
		.run();
	Ok(())
}
