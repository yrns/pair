//! A simple dynamics example.

use bevy::{
    input::mouse::MouseMotion,
    math::vec3,
    prelude::*,
    render::{
        camera::ScalingMode,
        render_resource::{
            AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDimension, TextureFormat,
        },
        texture::ImageSampler,
    },
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_startup_system(setup)
        .add_system(track_cursor)
        //.add_system(track_motion)
        .add_system(update_dynamics.after(track_cursor))
        .add_system(dynamics_window)
        .run();
}

/// A marker component for the tracking object.
#[derive(Component, Default)]
struct Tracking {
    target: Vec3,
    velocity: Vec3,
}

/// Remember the parameters so we can update them in real time.
#[derive(Component)]
struct Dynamic {
    f: f32,
    z: f32,
    r: f32,
    state: pair::SecondOrderDynamics<Vec3>,
}

impl Dynamic {
    pub fn new(f: f32, z: f32, r: f32) -> Self {
        Self {
            f,
            z,
            r,
            state: pair::SecondOrderDynamics::new(f, z, r, TRACKING_POS),
        }
    }
}

// Offset the two objects so we can see the difference in motion.
const TRACKING_POS: Vec3 = vec3(-3.0, 3.0, 0.0);
const DYNAMIC_POS: Vec3 = vec3(3.0, 3.0, 0.0);
const DYNAMIC_OFFSET: Vec3 = vec3(6.0, 0.0, 0.0);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // plane
    let plane = Mesh::from(shape::Plane::from_size(80.0));
    let grid = images.add(grid_texture());
    commands.spawn(PbrBundle {
        mesh: meshes.add(scale_uvs(plane, 16.0)),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(grid),
            perceptual_roughness: 0.3,
            metallic: 0.0,
            ..default()
        }),
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            0.0,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
        ..default()
    });

    // tracking object
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(
                shape::Icosphere {
                    radius: 0.8,
                    subdivisions: 5,
                }
                .try_into()
                .unwrap(),
            ),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_translation(TRACKING_POS),
            ..default()
        })
        .insert(Tracking {
            target: TRACKING_POS,
            ..default()
        });

    // dynamics object
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(
                shape::Icosphere {
                    radius: 1.0,
                    subdivisions: 5,
                }
                .try_into()
                .unwrap(),
            ),
            material: materials.add(Color::rgb(0.2, 0.7, 0.6).into()),
            transform: Transform::from_translation(DYNAMIC_POS),
            ..default()
        })
        // The dynamics are tracking the tracker internally. The offset is added post-update.
        .insert(Dynamic::new(2.5, 1.0, 1.0));

    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 16000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-10.0, 20.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        projection: OrthographicProjection {
            scale: 10.0,
            scaling_mode: ScalingMode::FixedVertical(2.0),
            ..default()
        }
        .into(),
        transform: Transform::from_xyz(0.0, 16.0, 16.0).looking_at(vec3(0.0, 3.0, 0.0), Vec3::Y),
        ..default()
    });
}

/// Tracks mouse motion and updates the tracking object.
#[allow(unused)]
fn track_motion(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Transform, &mut Tracking)>,
) {
    let dt = time.delta_seconds();
    let delta = if mouse_button_input.pressed(MouseButton::Left) {
        let d: Vec2 = mouse_events.iter().map(|m| m.delta).sum();
        let d = d * Vec2::new(1.1, 2.0) * 0.018;
        Some(vec3(d.x, 0.0, d.y))
    } else if mouse_button_input.just_released(MouseButton::Left) {
        Some(Vec3::ZERO)
    } else {
        None
    };

    if let Some(d) = delta {
        for (mut t, mut tracking) in query.iter_mut() {
            // Save target/velocity.
            let target = t.translation + d;
            tracking.target = target;
            tracking.velocity = if dt > 0.0 { d / dt } else { Vec3::ZERO };
            t.translation = target;
        }
    };
}

/// Moves the tracking object to the cursor location.
#[allow(unused)]
fn track_cursor(
    time: Res<Time>,
    mut cursor_moved: EventReader<CursorMoved>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut query: Query<(&mut Transform, &mut Tracking)>,
) {
    if let Some(c) = cursor_moved.iter().last() {
        for (camera, transform) in cameras.iter() {
            if let Some(p) = camera.viewport_to_world_2d(transform, c.position) {
                let ray = Ray {
                    origin: p.extend(transform.translation().z),
                    direction: transform.forward(),
                };

                if let Some(p) = intersect_tracking_plane(&ray) {
                    for (mut t, mut tracking) in query.iter_mut() {
                        let dt = time.delta_seconds();
                        tracking.velocity = if dt > 0.0 {
                            (p - tracking.target) / dt
                        } else {
                            Vec3::ZERO
                        };
                        tracking.target = p;
                        t.translation = p;
                    }
                }

                break;
            }
        }
    }
}

#[allow(unused)]
fn intersect_tracking_plane(ray: &Ray) -> Option<Vec3> {
    let dotn = Vec3::Y.dot(ray.direction);
    if dotn == 0.0 {
        None
    } else {
        let t = -((Vec3::Y.dot(ray.origin) - TRACKING_POS.y) / dotn);
        Some(ray.origin + ray.direction * t)
    }
}

/// Update dynamics object based on the tracking object's position and velocity.
fn update_dynamics(
    time: Res<Time>,
    tracking: Query<&Tracking>,
    mut dynamic: Query<(&mut Transform, &mut Dynamic)>,
) {
    // In this example there is only one.
    if let Some(Tracking { target, velocity }) = tracking.iter().next() {
        for (mut t, mut d) in dynamic.iter_mut() {
            t.translation = d
                .state
                .update(time.delta_seconds(), *target, Some(*velocity))
                + DYNAMIC_OFFSET;
        }
    }
}

fn grid_texture() -> Image {
    const GRID_SIZE: usize = 4;
    // #C3C5C9?
    let mut texture_data = [0xC3; GRID_SIZE * GRID_SIZE * 4 * 4];
    // #9E9EA3?
    let line = [0x9E; GRID_SIZE * 4];

    for y in 0..GRID_SIZE {
        let x = GRID_SIZE * 2 * 4 * y;
        texture_data[x..(x + line.len())].copy_from_slice(&line);
    }

    for y in GRID_SIZE..(GRID_SIZE * 2) {
        let x = GRID_SIZE * 4 + GRID_SIZE * 2 * 4 * y;
        texture_data[x..(x + line.len())].copy_from_slice(&line);
    }

    let mut image = Image::new_fill(
        Extent3d {
            width: (GRID_SIZE * 2) as u32,
            height: (GRID_SIZE * 2) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    );
    image.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
        address_mode_u: AddressMode::Repeat,
        address_mode_v: AddressMode::Repeat,
        //min_filter: FilterMode::Linear,
        //mag_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..default()
    });
    image
}

fn scale_uvs(mut mesh: Mesh, scale: f32) -> Mesh {
    match mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0).unwrap() {
        bevy::render::mesh::VertexAttributeValues::Float32x2(uvs) => {
            for uv in uvs {
                uv[0] *= scale;
                uv[1] *= scale;
            }
        }
        _ => (),
    }
    mesh
}

fn dynamics_window(mut contexts: EguiContexts, mut dynamics: Query<(Entity, &mut Dynamic)>) {
    for (entity, mut dynamic) in dynamics.iter_mut() {
        egui::Window::new(format!("dynamic {:?}", entity)).show(contexts.ctx_mut(), |ui| {
            let response = ui
                .add(egui::Slider::new(&mut dynamic.f, 0.0..=10.0).text("f (frequency)"))
                | ui.add(egui::Slider::new(&mut dynamic.z, 0.0..=10.0).text("ζ (damping)"))
                | ui.add(egui::Slider::new(&mut dynamic.r, -10.0..=10.0).text("r (response)"));

            if response.changed() {
                *dynamic = Dynamic::new(dynamic.f, dynamic.z, dynamic.r);
            }
        });
    }
}
