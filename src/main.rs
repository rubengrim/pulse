use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        mesh::{Indices, PrimitiveTopology},
    },
};
// use bevy_flycam::prelude::*;
use bevy_camera_operator::*;
use pulse::{path_tracer::*, PulsePlugin, PULSE_GRAPH};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        PulsePlugin,
        PulsePathTracerPlugin,
        CameraControllerPlugin,
        // FrameTimeDiagnosticsPlugin,
        // LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup)
    .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
) {
    // suppose Y-up right hand, and camera look from +z to -z
    let vertices = &[
        // Plane
        // ([-1.0, 1.0, -3.0], [0., 0., 1.0], [0., 0.]),
        // ([1.0, 1.0, -3.0], [0., 0., 1.0], [1.0, 0.]),
        // ([1.0, -1.0, -3.0], [0., 0., 1.0], [1.0, 1.0]),
        // ([-1.0, -1.0, -3.0], [0., 0., 1.0], [0., 1.0]),
        ([-2.0, 0.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([-1.0, 1.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([-2.0, 1.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([1.0, 0.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([2.0, 1.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([1.0, 1.0, -3.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 3, 4, 5,
        // 2, 3, 0,
    ]);

    // // note that we have to include the `Scene0` label
    // let my_gltf = asset_server.load("suzanne.glb#Scene0");

    // // to position our 3d model, simply use the Transform
    // // in the SceneBundle
    // commands.spawn(SceneBundle {
    //     scene: my_gltf,
    //     transform: Transform::from_xyz(0.0, 0.0, -5.0),
    //     ..Default::default()
    // });

    let mesh = Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices));

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(mesh),
    //     ..default()
    // });

    // let camera_target = commands
    //     .spawn(PbrBundle {
    //         mesh: mesh_assets.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         transform: Transform::from_scale(Vec3::new(3.0, 1.0, 1.0)),
    //         ..default()
    //     })
    //     .id();

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Torus {
            subdivisions_segments: 32,
            subdivisions_sides: 24,
            ..default()
        })),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgba(1.0, 0.6, 0.0, 1.0),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, -1.5),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 6000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // commands.spawn((
    //     Camera3dBundle::default(),
    //     FreeFlyCameraController::new(FreeFlyCameraControllerConfig::default()),
    // ));

    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
            transform: Transform::from_xyz(0.0, 0.0, 2.0),
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 3.1415 / 4.0,
                ..default()
            }),
            ..default()
        },
        PulsePathTracer::default(),
        FreeFlyCameraController::new(FreeFlyCameraControllerConfig::default()),
    ));
}
