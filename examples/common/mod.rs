use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use crate::Dynamic;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .add_systems(Startup, common_scene)
            .add_systems(Update, update_dynamics);
    }
}

pub fn update_dynamics(mut contexts: EguiContexts, mut dynamics: Query<(DebugName, &mut Dynamic)>) {
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

pub fn common_scene(
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
}

pub fn grid_texture() -> Image {
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

pub fn scale_uvs(mut mesh: Mesh, scale: f32) -> Mesh {
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
