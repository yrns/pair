//! A simple dynamics example.

mod common;

use bevy::{input::mouse::MouseMotion, prelude::*, render::camera::ScalingMode};

use common::Dynamics;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(common::Plugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // track_motion,
                track_cursor,
                update_dynamic,
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

// Offset the two objects so we can see the difference in motion.
const TRACKING_POS: Vec3 = Vec3::new(-3.0, 3.0, 0.0);
const DYNAMIC_POS: Vec3 = Vec3::new(3.0, 3.0, 0.0);
const DYNAMIC_OFFSET: Vec3 = Vec3::new(6.0, 0.0, 0.0);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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
        Dynamics::new(2.5, 1.0, 1.0, DYNAMIC_POS),
    ));

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
fn update_dynamic(
    time: Res<Time>,
    tracking: Query<(&Transform, &Velocity), With<Tracking>>,
    mut dynamic: Query<(&mut Transform, &mut Dynamics), Without<Tracking>>,
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
