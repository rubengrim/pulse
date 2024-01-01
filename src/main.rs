use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        mesh::{Indices, PrimitiveTopology},
    },
};
use bevy_camera_operator::*;
use pulse::{path_tracer::*, PulsePlugin, PULSE_GRAPH};

pub const RENDER_WITH_PULSE: bool = true;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }),
        PulsePlugin,
        PulsePathTracerPlugin,
        CameraControllerPlugin,
    ))
    .add_systems(Startup, setup)
    // .add_systems(Update, update_meshes)
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
        ([-1.0, 0.0, 1.0], [0., 0., 1.0], [0., 0.]),
        ([1.0, 0.0, 1.0], [0., 0., 1.0], [1.0, 0.]),
        ([1.0, 0.0, -1.0], [0., 0., 1.0], [1.0, 1.0]),
        ([-1.0, 0.0, -1.0], [0., 0., 1.0], [0., 1.0]),
        // ([-1.0, 1.0, 0.0], [0., 0., 1.0], [0., 0.]),
        // ([1.0, 1.0, 0.0], [0., 0., 1.0], [1.0, 0.]),
        // ([1.0, -1.0, 0.0], [0., 0., 1.0], [1.0, 1.0]),
        // ([-1.0, -1.0, 0.0], [0., 0., 1.0], [0., 1.0]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // 2, 3, 0,
    ]);

    let _mesh = Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices));

    // let m = mesh_assets.add(mesh);
    // commands.spawn(PbrBundle {
    //     mesh: m.clone(),
    //     transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
    //         .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, PI / 2.0)),
    //     ..default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: m,
    //     transform: Transform::from_xyz(1.0, 0.0, -4.0),
    //     ..default()
    // });

    // note that we have to include the `Scene0` label
    let monkey = asset_server.load("monkey_smooth.glb#Scene0");

    let step_size = 3.0;
    let resolution = 1;
    for x in 0..resolution {
        for z in 0..resolution {
            let transform = Transform::from_xyz(x as f32 * step_size, 0.0, z as f32 * step_size)
                .with_scale(Vec3::new(1.0, 2.0, 1.0));
            commands.spawn(SceneBundle {
                scene: monkey.clone(),
                transform,
                ..Default::default()
            });
        }
    }

    // commands.spawn(SceneBundle {
    //     scene: my_gltf,
    //     transform: Transform::from_xyz(-4.0, 0.0, -1.0),
    //     ..Default::default()
    // });

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(
            Mesh::try_from(shape::Icosphere {
                radius: 0.9,
                subdivisions: 5,
            })
            .unwrap(),
        ),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            emissive: Color::BLACK,
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, -3.0)),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(
            Mesh::try_from(shape::Icosphere {
                radius: 2.0,
                subdivisions: 2,
            })
            .unwrap(),
        ),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgba(1.0, 0.0, 0.0, 1.0),
            emissive: Color::WHITE,
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(-4.0, 0.0, -2.0)),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Plane {
            size: 10.0,
            subdivisions: 0,
        })),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgba(0.0, 1.0, 0.0, 1.0),
            emissive: Color::rgba(0.0, 0.0, 0.0, 0.0),
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        ..default()
    });

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     transform: Transform::from_translation(Vec3::new(-2.0, 0.0, -3.0)),
    //     ..default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Torus {
    //         subdivisions_segments: 32,
    //         subdivisions_sides: 24,
    //         ..default()
    //     })),
    //     material: mat_assets.add(StandardMaterial {
    //         base_color: Color::rgba(1.0, 0.6, 0.0, 1.0),
    //         ..default()
    //     }),
    //     transform: Transform::from_xyz(0.0, 0.0, -1.5),
    //     ..default()
    // });

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

    if RENDER_WITH_PULSE {
        commands.spawn((
            Camera3dBundle {
                camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
                transform: Transform::from_xyz(0.0, 0.0, 10.0),
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
    } else {
        commands.spawn((
            Camera3dBundle::default(),
            FreeFlyCameraController::new(FreeFlyCameraControllerConfig::default()),
        ));
    }
}

pub fn update_meshes(mut meshes_q: Query<&mut Transform, With<Handle<Mesh>>>, time: Res<Time>) {
    for mut transform in meshes_q.iter_mut() {
        let e = Vec3::new(0.07, 0.02, 0.02) * time.delta_seconds() * 10.0;
        transform.rotate(Quat::from_euler(EulerRot::XYZ, e.x, e.y, e.z));
    }
}
