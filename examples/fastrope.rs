//! A simple rope. This is a rehash of this tweet: https://x.com/t3ssel8r/status/1470039981502922752

mod common;

use bevy::prelude::*;
use bevy_mod_picking::DefaultPickingPlugins;
use bevy_transform_gizmo::*;

use common::Dynamics;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((DefaultPickingPlugins, TransformGizmoPlugin::default()))
        .add_plugins(common::Plugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update_rope)
        .run();
}

/// A rope.
#[derive(Component, Debug)]
struct Rope {
    length: f32,
    midpoint: Entity,
    end: Entity,
}

#[derive(Component)]
struct Point;

const CUBE_SIZE: f32 = 1.2;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let start = Vec3::new(-3.0, 6.0, 0.0);
    let end = Vec3::new(3.0, 6.0, 0.0);
    let midpoint = (start + end) * 0.5;
    let local_offset = Vec3::X * CUBE_SIZE * 0.5;

    let midpoint_id = commands
        .spawn((
            Point,
            SpatialBundle::from_transform(Transform::from_translation(midpoint)),
        ))
        .id();

    // The endpoint will be attached to the left side of the second cube.
    let end_id = commands
        .spawn((
            Point,
            SpatialBundle::from_transform(Transform::from_translation(-local_offset)),
        ))
        .id();

    // cube1
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::from_size(Vec3::splat(CUBE_SIZE)).mesh()),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
                transform: Transform::from_translation(start),
                ..default()
            },
            bevy_mod_picking::PickableBundle::default(),
            bevy_transform_gizmo::GizmoTransformable,
        ))
        .with_children(|p| {
            // The rope is attached to the side of the first cube.
            p.spawn((
                Rope {
                    length: 8.0,
                    midpoint: midpoint_id,
                    end: end_id,
                },
                Name::new("Rope settings"),
                Dynamics::new(3.0, 0.5, 2.0, midpoint),
                SpatialBundle::from_transform(Transform::from_translation(local_offset)),
            ));
        });

    // cube2
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::from_size(Vec3::splat(CUBE_SIZE)).mesh()),
                material: materials.add(Color::rgb(0.2, 0.7, 0.6)),
                transform: Transform::from_translation(end),
                ..default()
            },
            bevy_mod_picking::PickableBundle::default(),
            bevy_transform_gizmo::GizmoTransformable,
        ))
        .push_children(&[end_id]);

    // camera
    commands.spawn((
        Camera3dBundle {
            // projection: OrthographicProjection {
            //     scale: 10.0,
            //     scaling_mode: ScalingMode::FixedVertical(2.0),
            //     ..default()
            // }
            // .into(),
            transform: Transform::from_xyz(0.0, 18.0, 16.0)
                .looking_at(Vec3::new(0.0, 6.0, 0.0), Vec3::Y),
            ..default()
        },
        bevy_transform_gizmo::GizmoPickSource::default(),
    ));
}

/// Draw a rope between two points.
fn update_rope(
    time: Res<Time>,
    mut ropes: Query<(&Rope, &GlobalTransform, &mut Dynamics), Without<Point>>,
    mut points: Query<(&GlobalTransform, &mut Transform), With<Point>>,
    mut gizmos: Gizmos,
) {
    let dt = time.delta_seconds();

    for (rope, start, mut dynamic) in ropes.iter_mut() {
        let start = start.translation();
        let end = points
            .get(rope.end)
            .expect("endpoint exists")
            .0
            .translation();
        let (_, mut mid_t) = points.get_mut(rope.midpoint).expect("midpoint exists");

        let midpoint = (start + end) * 0.5;
        let slack = rope.length - start.distance(end);
        let drop = midpoint - Vec3::new(0.0, slack.max(0.0), 0.0);

        // The red lines display a fixed midpoint and drop point, which depends on the slack in the rope.
        gizmos.line(start, end, Color::RED);
        gizmos.line(midpoint, drop, Color::RED);
        gizmos.circle(drop, Direction3d::Y, 0.1, Color::RED);

        // Technically, we don't need to update the midpoint to draw the rope, but it's there if you
        // want to attach something to it.
        if dt > 0.0 {
            mid_t.translation = dynamic.state.update(dt, drop, None);
            let bezier = raise(start, mid_t.translation, end);
            gizmos.linestrip(bezier.to_curve().iter_positions(64), Color::BLACK);
        }
    }
}

// Make a cubic bezier from a quadratic.
fn raise(p0: Vec3, p1: Vec3, p2: Vec3) -> CubicBezier<Vec3> {
    CubicBezier::new([[
        p0,
        p0 + (2.0 / 3.0) * (p1 - p0),
        p2 + (2.0 / 3.0) * (p1 - p2),
        p2,
    ]])
}
