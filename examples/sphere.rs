//! A simple dynamics example.

use bevy::{input::mouse::MouseMotion, prelude::*, render::camera::ScalingMode};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                dynamics_window,
                // track_motion,
                track_cursor,
                update_dynamics,
            )
                .chain(),
        )
        .run();
}

/// A marker component for the tracking object.
#[derive(Component)]
struct Tracking;

/// Velocity component.
#[derive(Component, Default)]
struct Velocity(Vec3);

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
const TRACKING_POS: Vec3 = Vec3::new(-3.0, 3.0, 0.0);
const DYNAMIC_POS: Vec3 = Vec3::new(3.0, 3.0, 0.0);
const DYNAMIC_OFFSET: Vec3 = Vec3::new(6.0, 0.0, 0.0);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // ground plane grid
    let plane = scale_uvs(Plane3d::default().mesh().size(80.0, 80.0).build(), 16.0);
    let grid = images.add(grid_texture());
    commands.spawn(PbrBundle {
        mesh: meshes.add(plane),
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
            mesh: meshes.add(Sphere::new(0.8).mesh().ico(5).unwrap()),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
            transform: Transform::from_translation(TRACKING_POS),
            ..default()
        })
        .insert((Tracking, Velocity::default()));

    // dynamics object
    commands.spawn((
        Name::new("Dynamics settings"),
        PbrBundle {
            mesh: meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap()),
            material: materials.add(Color::rgb(0.2, 0.7, 0.6)),
            transform: Transform::from_translation(DYNAMIC_POS),
            ..default()
        },
        // The dynamics are tracking the tracker internally. The offset is added post-update.
        Dynamic::new(2.5, 1.0, 1.0),
    ));

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
        transform: Transform::from_xyz(0.0, 16.0, 16.0)
            .looking_at(Vec3::new(0.0, 3.0, 0.0), Vec3::Y),
        ..default()
    });
}

/// Tracks mouse motion and updates the tracking object.
#[allow(unused)]
fn track_motion(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut Transform, &mut Velocity), With<Tracking>>,
) {
    let dt = time.delta_seconds();
    let delta = if mouse_button_input.pressed(MouseButton::Left) {
        let d: Vec2 = mouse_events.read().map(|m| m.delta).sum();
        let d = d * Vec2::new(1.1, 2.0) * 0.018;
        Some(Vec3::new(d.x, 0.0, d.y))
    } else if mouse_button_input.just_released(MouseButton::Left) {
        Some(Vec3::ZERO)
    } else {
        None
    };

    if let Some(d) = delta {
        for (mut t, mut v) in query.iter_mut() {
            // Save target/velocity.
            let target = t.translation + d;
            v.0 = if dt > 0.0 { d / dt } else { Vec3::ZERO };
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
    mut query: Query<(&mut Transform, &mut Velocity), With<Tracking>>,
) {
    if let Some(c) = cursor_moved.read().last() {
        let dt = time.delta_seconds();

        for (camera, transform) in cameras.iter() {
            if let Some(p) = camera.viewport_to_world_2d(transform, c.position) {
                let ray = Ray3d {
                    origin: p.extend(transform.translation().z),
                    direction: Direction3d::new(transform.forward()).unwrap(),
                };

                if let Some(p) = intersect_tracking_plane(&ray) {
                    for (mut t, mut v) in query.iter_mut() {
                        if dt != 0.0 {
                            v.0 = (p - t.translation) / dt;
                        }
                        t.translation = p;
                    }
                }

                break;
            }
        }
    }
}

#[allow(unused)]
fn intersect_tracking_plane(ray: &Ray3d) -> Option<Vec3> {
    let dotn = Vec3::Y.dot(*ray.direction);
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
    tracking: Query<(&Transform, &Velocity), With<Tracking>>,
    mut dynamic: Query<(&mut Transform, &mut Dynamic), Without<Tracking>>,
) {
    // In this example there is only one.
    if let Some((t0, v)) = tracking.iter().next() {
        for (mut t, mut d) in dynamic.iter_mut() {
            t.translation = d
                .state
                .update(time.delta_seconds(), t0.translation, Some(v.0))
                + DYNAMIC_OFFSET;
        }
    }
}

fn grid_texture() -> Image {
    use bevy::render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    };

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
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        //min_filter: FilterMode::Linear,
        //mag_filter: FilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
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

fn dynamics_window(mut contexts: EguiContexts, mut dynamics: Query<(DebugName, &mut Dynamic)>) {
    for (name, mut dynamic) in dynamics.iter_mut() {
        egui::Window::new(format!("{:?}", name)).show(contexts.ctx_mut(), |ui| {
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
