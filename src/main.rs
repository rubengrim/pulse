use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        mesh::{Indices, PrimitiveTopology},
    },
};
use bevy_flycam::prelude::*;
use pulse::{path_tracer::*, PulsePlugin, PULSE_GRAPH};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        PulsePlugin,
        PulsePathTracerPlugin,
        NoCameraPlayerPlugin,
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands, mut mesh_assets: ResMut<Assets<Mesh>>) {
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

    let mesh = Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices));

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(mesh),
    //     ..default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     ..default()
    // });

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Torus::default())),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
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
        FlyCam,
    ));
}
