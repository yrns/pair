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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(track_mouse)
        .add_system(update_dynamics.after(track_mouse))
        .run();
}

/// A marker component for the tracking object.
#[derive(Component, Default)]
struct Tracking {
    target: Vec3,
    velocity: Vec3,
}

#[derive(Component)]
struct Dynamic(pair::SecondOrderDynamics);

// Offset the two objects so we can see the difference in motion.
const TRACKING_POS: Vec3 = vec3(-4.0, 3.0, 0.0);
const DYNAMIC_POS: Vec3 = vec3(4.0, 3.0, 0.0);
const DYNAMIC_OFFSET: Vec3 = vec3(8.0, 0.0, 0.0);

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
        .insert(Dynamic(pair::SecondOrderDynamics::new(
            9.5,
            0.9,
            10.0,
            // The dynamics are tracking the tracker internally. The offset is added post-update.
            TRACKING_POS,
        )));

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
fn track_mouse(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut Transform, &mut Tracking)>,
) {
    let dt = time.delta_seconds();
    let delta = if mouse_button_input.pressed(MouseButton::Left) {
        let d: Vec2 = mouse_events.iter().map(|m| m.delta).sum();
        let d = d * Vec2::new(1.0, 1.8) * dt;
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

/// Update dynamics object based on the tracking object's position and velocity.
fn update_dynamics(
    time: Res<Time>,
    tracking: Query<&Tracking>,
    mut dynamic: Query<(&mut Transform, &mut Dynamic)>,
) {
    // In this example there is only one.
    if let Some(Tracking { target, velocity }) = tracking.iter().next() {
        for (mut t, mut d) in dynamic.iter_mut() {
            t.translation =
                d.0.update(time.delta_seconds(), *target, Some(*velocity)) + DYNAMIC_OFFSET;
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
