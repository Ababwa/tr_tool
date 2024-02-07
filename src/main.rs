use std::env::args;
use bevy::{
	app::{App, AppExit, PluginGroup, Startup, Update}, asset::{Asset, Assets, Handle}, core_pipeline::{
		clear_color::ClearColorConfig,
		core_3d::{Camera3d, Camera3dBundle},
	}, ecs::{
		component::Component,
		entity::Entity,
		event::EventWriter,
		query::With,
		system::{Commands, Query, Res, ResMut, Resource},
	}, hierarchy::DespawnRecursiveExt, input::{keyboard::KeyCode, mouse::MouseButton, Input}, pbr::{Material, MaterialMeshBundle, MaterialPlugin}, reflect::{Reflect, TypePath, TypeUuid}, render::{
		color::Color,
		mesh::Mesh,
		render_resource::{
			AsBindGroup, Extent3d, PrimitiveTopology, ShaderRef, TextureDescriptor,
			TextureDimension, TextureFormat, TextureUsages,
		},
		texture::{Image, ImageSampler},
	}, time::Time, window::{CursorGrabMode, Window, WindowPlugin, WindowResolution}, DefaultPlugins
};
use glam_traits::glam::{vec3, I16Vec3, Mat3, Vec2, Vec3, Vec3Swizzles};
use leafwing_input_manager::{
	prelude::{ActionState, DualAxis, InputManagerPlugin, InputMap},
	Actionlike,
};
use smooth_bevy_cameras::{
	LookAngles, LookTransform, LookTransformBundle, LookTransformPlugin, Smoother,
};
use tr_reader::tr4;
use tr_tool::{geom::{MinMax, VecMinMax}, load::{self, LevelRenderData}, vec_convert::{ToBevy, ToGlam}, vtx_attr::VtxAttr};

#[derive(Actionlike, Clone, Reflect, Hash, PartialEq, Eq)]
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

#[derive(TypeUuid, AsBindGroup, Clone, Asset, TypePath)]
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

fn add_vertices<const N: usize>(
	pos: &mut Vec<Vec3>,
	tex: &mut Vec<Vec2>,
	room_verts: &[Vec3],
	object_textures: &[[Vec2; 4]],
	faces: &[tr4::RoomFace<N>],
	indices: &[usize],
) {
	for &tr4::RoomFace { texture_details, vertex_ids } in faces {
		let tex_id = texture_details.texture_id() as usize;
		for &i in indices {
			pos.push(room_verts[vertex_ids[i] as usize]);
			tex.push(object_textures[tex_id][i]);
		}
	}
}

fn build_scene(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<FaceMaterial>,
	image: &Handle<Image>,
	object_textures: &[[Vec2; 4]],
	rooms: &[tr4::Room],
	old_scene: Option<Query<Entity, With<Scene>>>,
) -> MinMax<Vec3> {
	if let Some(old_scene) = old_scene {
		for entity in &old_scene {
			commands.entity(entity).despawn_recursive();
		}
	}
	let mut bounds = MinMax { min: Vec3::INFINITY, max: Vec3::NEG_INFINITY };
	for room in rooms {
		let mut room_verts = Vec::with_capacity(room.vertices.len());
		for &tr4::RoomVertex { vertex: I16Vec3 { x, y, z }, .. } in room.vertices.iter() {
			let v = vec3((x as i32 + room.x) as f32, y as f32, (z as i32 + room.z) as f32) / 1024.0;
			bounds.update(v);
			room_verts.push(v);
		}
		let num_verts = (room.triangles.len() * 3) + (room.quads.len() * 6);
		let mut mesh_verts = Vec::with_capacity(num_verts);
		let mut mesh_tex_coords = Vec::with_capacity(num_verts);
		add_vertices(&mut mesh_verts, &mut mesh_tex_coords, &room_verts, object_textures, &room.triangles, &[2, 1, 0]);
		add_vertices(&mut mesh_verts, &mut mesh_tex_coords, &room_verts, object_textures, &room.quads, &[2, 1, 0, 0, 3, 2]);
		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VtxAttr(mesh_verts));
		mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VtxAttr(mesh_tex_coords));
		commands
			.spawn(MaterialMeshBundle {
				mesh: meshes.add(mesh),
				material: materials.add(FaceMaterial { texture: image.clone() }),
				..Default::default()
			})
			.insert(Scene);
	}
	bounds
}

const CAM_ROT_DIVISOR: f32 = 200.0;

fn update_look_target(look_transform: &mut LookTransform, action_state: &ActionState<CameraAction>) {
	let [x, y] = action_state.axis_pair(CameraAction::Look).unwrap().xy().to_array();
	let mut look_angles = LookAngles::from_vector(look_transform.look_direction().unwrap());
	look_angles.add_yaw(x / CAM_ROT_DIVISOR);
	look_angles.add_pitch(y / CAM_ROT_DIVISOR);
	look_transform.target = look_transform.eye + look_angles.unit_vector();
}

const CAM_SPEED_SLOW: f32 = 10.0;
const CAM_SPEED_FAST: f32 = 30.0;

const ACTION_DIRECTION: [(CameraAction, Vec3); 6] = [
	(CameraAction::Forward, Vec3::X),
	(CameraAction::Backward, Vec3::NEG_X),
	(CameraAction::Left, Vec3::Z),
	(CameraAction::Right, Vec3::NEG_Z),
	(CameraAction::Up, Vec3::NEG_Y),
	(CameraAction::Down, Vec3::Y),
];

fn move_delta(Vec2 { x, y }: Vec2, action_state: &ActionState<CameraAction>, time: &Time) -> Vec3 {
	let speed = if action_state.pressed(CameraAction::Shift) {
		CAM_SPEED_FAST
	} else {
		CAM_SPEED_SLOW
	};
	let look_rotation = Mat3::from_cols(Vec3::new(x, 0.0, y), Vec3::Y, Vec3::new(-y, 0.0, x));
	let movement = ACTION_DIRECTION
		.into_iter()
		.filter_map(|(action, dir)| action_state.pressed(action).then_some(dir))
		.sum::<Vec3>();
	(time.delta_seconds() * speed) * (look_rotation * movement)
}

fn toggle_camera_control(window: &mut Window, camera_control: &mut bool) {
	window.cursor.visible = *camera_control;
	window.cursor.grab_mode = if *camera_control {
		CursorGrabMode::None
	} else {
		CursorGrabMode::Confined
	};
	*camera_control ^= true;
}

fn camera_control(
	action_state: Res<ActionState<CameraAction>>,
	mut windows: Query<&mut Window>,
	mut look_transform: Query<&mut LookTransform>,
	camera_control: ResMut<CameraControl>,
	time: Res<Time>,
) {
	let action_state = action_state.as_ref();
	let CameraControl(camera_control) = camera_control.into_inner();
	if action_state.just_pressed(CameraAction::Toggle) {
		toggle_camera_control(windows.single_mut().into_inner(), camera_control);
	}
	if *camera_control {
		let look_transform = look_transform.single_mut().into_inner();
		update_look_target(look_transform, action_state);
		if let Some(look_dir) = look_transform.look_direction().unwrap().to_glam().xz().try_normalize() {
			let delta = move_delta(look_dir, action_state, time.as_ref());
			look_transform.eye += delta.to_bevy();
			look_transform.target += delta.to_bevy();
		}
	}
}

#[derive(Resource)]
struct LevelPath(String);

fn setup(
	mut commands: Commands,
	meshes: ResMut<Assets<Mesh>>,
	materials: ResMut<Assets<FaceMaterial>>,
	images: ResMut<Assets<Image>>,
	level_path: Res<LevelPath>,
) {
	let images = images.into_inner();
	let meshes = meshes.into_inner();
	let materials = materials.into_inner();
	let LevelPath(level_path) = level_path.into_inner();
	let LevelRenderData { atlas_size, atlas_data, texture_coords, rooms } = load::load_level(level_path);
	let image = Image {
		texture_descriptor: TextureDescriptor {
			size: Extent3d {
				width: atlas_size.x as u32,
				height: atlas_size.y as u32,
				depth_or_array_layers: 1,
			},
			dimension: TextureDimension::D2,
			format: TextureFormat::Bgra8UnormSrgb,
			label: None,
			mip_level_count: 1,
			sample_count: 1,
			usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
			view_formats: &[],
		},
		data: atlas_data,
		sampler: ImageSampler::nearest(),
		..Default::default()
	};
	let image = images.add(image);
	let MinMax { min, max } = build_scene(
		&mut commands,
		meshes,
		materials,
		&image,
		&texture_coords,
		&rooms,
		None,
	);
	commands.spawn(Camera3dBundle {
		camera_3d: Camera3d {
			clear_color: ClearColorConfig::Custom(Color::rgb(0.1, 0.3, 0.2)),
			..Default::default()
		},
		..Default::default()
	}).insert(LookTransformBundle {
		transform: LookTransform::new(min.to_bevy(), max.to_bevy(), Vec3::NEG_Y.to_bevy()),
		smoother: Smoother::new(0.5),
	});
	commands.remove_resource::<LevelPath>();
}

fn escape_quit(keyboard: Res<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
	if keyboard.pressed(KeyCode::Key1) {//escape key is broken, using 1 until new hardware
		exit.send(AppExit);
	}
}

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

fn main() {
	let level_path = args().skip(1).next().expect("Path to .tr4 file must be provided");
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "TR Reader".to_owned(),
				resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
				..Default::default()
			}),
			..Default::default()
		}))
		.add_plugins(LookTransformPlugin)
		.add_plugins(InputManagerPlugin::<CameraAction>::default())
		.add_plugins(MaterialPlugin::<FaceMaterial>::default())
		.insert_resource(CameraControl(false))
		.insert_resource(ActionState::<CameraAction>::default())
		.insert_resource(
			InputMap::<CameraAction>::default()
				.insert(DualAxis::mouse_motion(), CameraAction::Look)
				.insert(MouseButton::Right, CameraAction::Toggle)
				.insert(KeyCode::W, CameraAction::Forward)
				.insert(KeyCode::A, CameraAction::Left)
				.insert(KeyCode::S, CameraAction::Backward)
				.insert(KeyCode::D, CameraAction::Right)
				.insert(KeyCode::Q, CameraAction::Up)
				.insert(KeyCode::E, CameraAction::Down)
				.insert(KeyCode::ShiftLeft, CameraAction::Shift)
				.build(),
		)
		.insert_resource(LevelPath(level_path))
		.add_systems(Startup, setup)
		.add_systems(Update, (camera_control, escape_quit))
		.run();
}
