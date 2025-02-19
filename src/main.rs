use std::f32::consts::PI;

use bevy::{
    core_pipeline::{
        experimental::taa::{
            TemporalAntiAliasBundle, TemporalAntiAliasPlugin, TemporalAntiAliasSettings,
        },
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::Tonemapping,
    },
    pbr::{
        DefaultOpaqueRendererMethod, DirectionalLightShadowMap, OpaqueRendererMethod, PbrPlugin,
    },
    prelude::*,
    render::camera::TemporalJitter,
    window::WindowResolution,
};
use bevy_camera_operator::*;
use pulse::{
    path_tracer::*,
    pulse::{PulseCamera, PulseRealtimePlugin},
    PulsePlugin,
};

pub enum RenderingEngine {
    Bevy,
    PulsePathTracer,
    PulseRealtime,
}

pub const RENDERING_ENGINE: RenderingEngine = RenderingEngine::PulseRealtime;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::Immediate,
                    // resolution: WindowResolution::new(400.0, 400.0),
                    ..default()
                }),
                ..default()
            })
            .set(PbrPlugin {
                add_default_deferred_lighting_plugin: true,
                ..default()
            }),
        PulsePlugin,
        PulsePathTracerPlugin,
        CameraControllerPlugin,
    ))
    .add_systems(Startup, setup)
    .insert_resource(DefaultOpaqueRendererMethod::deferred())
    .insert_resource(Msaa::Off)
    .insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    .insert_resource(DirectionalLightShadowMap { size: 4096 })
    .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
) {
    let cornell = asset_server.load("cornell_statue.glb#Scene0");
    commands.spawn(SceneBundle {
        scene: cornell.clone(),
        // transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, -PI / 2.0, 0.0)),
        ..Default::default()
    });

    // let monkey = asset_server.load("monkey_orange.glb#Scene0");
    // commands.spawn(SceneBundle {
    //     scene: monkey.clone(),
    //     transform: Transform::from_scale(Vec3::splat(0.1)),
    //     ..Default::default()
    // });

    // Light
    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Plane::from_size(0.9))),
        transform: Transform::from_translation(Vec3::new(0.0, 0.98, 0.0))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, PI, 0.0, 0.0)),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.9, 0.7),
            emissive: Color::rgb(1.0, 0.8, 0.4) * 10.0,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            opaque_render_method: OpaqueRendererMethod::Deferred,
            ..default()
        }),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 1.0, 0.0)),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 3.0),
            camera: Camera {
                hdr: true,
                ..default()
            },
            tonemapping: Tonemapping::None,
            ..default()
        },
        FreeFlyCameraController::new(FreeFlyCameraControllerConfig {
            pitch_update_speed: 0.1,
            yaw_update_speed: 0.1,
            ..default()
        }),
        PulsePathTracerCamera {
            // resolution: Some(UVec2::new(720, 480)),
            resolution: Some(UVec2::new(1080, 720)),
            ..default()
        },
        DepthPrepass,
        DeferredPrepass,
    ));

    // Render without pulse
    // commands.spawn((
    //     Camera3dBundle {
    //         transform: Transform::from_xyz(0.0, 0.0, 3.0),
    //         camera: Camera {
    //             hdr: true,
    //             ..default()
    //         },
    //         tonemapping: Tonemapping::None,
    //         ..default()
    //     },
    //     DepthPrepass,
    //     DeferredPrepass,
    // ));
}
