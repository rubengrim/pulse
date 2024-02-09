use rand::Rng;
use std::f32::consts::PI;

use bevy::{
    asset::LoadedFolder,
    core_pipeline::{
        experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin},
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
        tonemapping::Tonemapping,
    },
    pbr::{
        DefaultOpaqueRendererMethod, DirectionalLightShadowMap, Mesh3d, OpaqueRendererMethod,
        PbrPlugin,
    },
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    },
    window::WindowResolution,
};
use bevy_camera_operator::*;
use pulse::{
    path_tracer::*,
    pulse::{PulseGI, PulseRealtimePlugin},
    PulsePlugin,
};

pub enum RenderingEngine {
    Bevy,
    PulsePathTracer,
    PulseRealtime,
}

#[derive(Component)]
pub struct MeshToSample;

const NUM_PARTICLES: u32 = 100;

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
        // PulsePlugin,
        // PulseRealtimePlugin,
        // PulsePathTracerPlugin,
        CameraControllerPlugin,
        TemporalAntiAliasPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, sample_mesh)
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
    let cornell = asset_server.load("cornell_no_light.glb#Scene0");
    commands.spawn(SceneBundle {
        scene: cornell.clone(),
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, -PI / 2.0, 0.0)),
        ..Default::default()
    });

    let monkey = asset_server.load("monkey_smooth.glb#Scene0");
    commands.spawn(SceneBundle {
        scene: monkey.clone(),
        transform: Transform::from_scale(Vec3::splat(0.1)),
        ..Default::default()
    });

    // let particle_test = asset_server.load("particle_test.glb#Scene0");
    // commands.spawn(SceneBundle {
    //     scene: particle_test.clone(),
    //     // transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, -PI / 2.0, 0.0)),
    //     ..Default::default()
    // });

    // let statue = asset_server.load("statue.glb#Scene0");
    // commands.spawn((
    //     SceneBundle {
    //         scene: statue.clone(),
    //         // transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.0, -PI / 2.0, 0.0)),
    //         ..Default::default()
    //     },
    //     MeshToSample,
    // ));

    // commands.spawn((PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube::new(1.0))),
    //     // mesh: mesh_assets.add(Mesh::from(shape::UVSphere {
    //     //     radius: 1.0,
    //     //     ..default()
    //     // })),
    //     material: mat_assets.add(StandardMaterial {
    //         base_color: Color::WHITE,
    //         ..default()
    //     }),
    //     ..default()
    // },));

    // commands.spawn((PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube::new(0.3))),
    //     // mesh: mesh_assets.add(Mesh::from(shape::UVSphere {
    //     //     radius: 0.2,
    //     //     ..default()
    //     // })),
    //     material: mat_assets.add(StandardMaterial {
    //         emissive: Color::rgb(0.0, 0.0, 1.0) * 1.0,
    //         ..default()
    //     }),
    //     transform: Transform::from_translation(Vec3::new(2.0, 1.0, 0.0)),
    //     ..default()
    // },));

    // commands.spawn((PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube::new(0.3))),
    //     // mesh: mesh_assets.add(Mesh::from(shape::UVSphere {
    //     //     radius: 0.2,
    //     //     ..default()
    //     // })),
    //     material: mat_assets.add(StandardMaterial {
    //         emissive: Color::rgb(1.0, 0.0, 0.0) * 0.7,
    //         ..default()
    //     }),
    //     transform: Transform::from_translation(Vec3::new(-0.3, 0.0, 3.0)),
    //     ..default()
    // },));

    commands.spawn(PbrBundle {
        mesh: mesh_assets.add(Mesh::from(shape::Plane::from_size(0.7))),
        transform: Transform::from_translation(Vec3::new(0.0, 0.98, 0.0))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, PI, 0.0, 0.0)),
        material: mat_assets.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.9, 0.7),
            emissive: Color::rgb(1.0, 0.8, 0.4) * 2.0,
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.0,
            opaque_render_method: OpaqueRendererMethod::Deferred,
            ..default()
        }),
        ..default()
    });

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube::default())),
    //     material: mat_assets.add(StandardMaterial {
    //         base_color: Color::rgb(1.0, 1.0, 1.0),
    //         emissive: Color::rgb(0.0, 0.0, 0.0),
    //         perceptual_roughness: 1.0,
    //         metallic: 0.0,
    //         reflectance: 0.0,
    //         opaque_render_method: OpaqueRendererMethod::Deferred,
    //         ..default()
    //     }),
    //     ..default()
    // });

    // commands.spawn(PbrBundle {
    //     mesh: mesh_assets.add(Mesh::from(shape::Cube::default())),
    //     transform: Transform::from_translation(Vec3::new(-0.4, -0.2, 2.0)),
    //     material: mat_assets.add(StandardMaterial {
    //         base_color: Color::rgb(1.0, 0.0, 1.0),
    //         emissive: Color::rgb(1.0, 1.0, 1.0),
    //         perceptual_roughness: 1.0,
    //         metallic: 0.0,
    //         reflectance: 0.0,
    //         opaque_render_method: OpaqueRendererMethod::Deferred,
    //         ..default()
    //     }),
    //     ..default()
    // });

    // commands.spawn(PointLightBundle {
    //     point_light: PointLight {
    //         shadows_enabled: false,
    //         intensity: 500.0,
    //         ..default()
    //     },
    //     transform: Transform::from_xyz(0.3, 0.8, 0.0),
    //     ..default()
    // });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight::default(),
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 1.0, 0.0)),
        ..default()
    });

    // commands.spawn((
    //     PbrBundle {
    //         mesh: mesh_assets.add(Mesh::from(shape::Cylinder::default())),
    //         material: mat_assets.add(StandardMaterial {
    //             base_color: Color::WHITE,
    //             ..default()
    //         }),
    //         ..default()
    //     },
    //     MeshToSample,
    // ));

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
        PulseGI,
        FreeFlyCameraController::new(FreeFlyCameraControllerConfig {
            pitch_update_speed: 0.1,
            yaw_update_speed: 0.1,
            ..default()
        }),
        // DepthPrepass,
        // MotionVectorPrepass,
        DeferredPrepass,
        // NormalPrepass,
        TemporalAntiAliasBundle::default(),
    ));

    // }
}

#[derive(Component)]
struct Particle;

fn sample_mesh(
    // mesh_q: Query<(&Handle<Mesh>, &GlobalTransform), With<MeshToSample>>,
    mesh_q: Query<(&Handle<Mesh>, &GlobalTransform)>,
    key_input: Res<Input<KeyCode>>,
    mut commands: Commands,
    particle_q: Query<Entity, With<Particle>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mat_assets: ResMut<Assets<StandardMaterial>>,
) {
    if key_input.just_pressed(KeyCode::Return) {
        let (mesh_handle, transform) = mesh_q.get_single().unwrap();
        let mesh = mesh_assets.get(mesh_handle).unwrap();

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(VertexAttributeValues::as_float3)
            .unwrap()
            .iter()
            .map(|p| Vec3::from_array(*p))
            .collect::<Vec<Vec3>>();

        let indices: Vec<u32> = match mesh.indices() {
            Some(Indices::U16(values)) => values.iter().map(|v| *v as u32).collect::<Vec<u32>>(),
            Some(Indices::U32(values)) => values.clone(),
            None => {
                error!("Cannot load mesh with no index buffer.");
                return;
            }
        };

        let obj_to_world = transform.compute_matrix();

        let mut tris = vec![];
        for i_0 in 0..(indices.len() / 3) {
            let i_0 = i_0 * 3;
            let v_0 = indices[i_0] as usize;
            let v_1 = indices[i_0 + 1] as usize;
            let v_2 = indices[i_0 + 2] as usize;
            let p_0 = positions[v_0];
            let p_1 = positions[v_1];
            let p_2 = positions[v_2];
            // Convert to world space
            let p_world_0 = obj_to_world * Vec4::new(p_0.x, p_0.y, p_0.z, 1.0);
            let p_world_1 = obj_to_world * Vec4::new(p_1.x, p_1.y, p_1.z, 1.0);
            let p_world_2 = obj_to_world * Vec4::new(p_2.x, p_2.y, p_2.z, 1.0);
            tris.push([
                p_world_0.xyz() / p_world_0.w,
                p_world_1.xyz() / p_world_1.w,
                p_world_2.xyz() / p_world_2.w,
            ]);
        }

        let mut areas = vec![];
        for t in tris.iter() {
            // Heron's formula for triangle area
            let a = (t[1] - t[0]).length();
            let b = (t[2] - t[0]).length();
            let c = (t[2] - t[1]).length();
            let s = 0.5 * (a + b + c);
            let area = (s * (s - a) * (s - b) * (s - c)).sqrt();
            areas.push(area);
        }

        let total_area: f32 = areas.iter().sum();
        let normalized_areas = areas.iter().map(|a| a / total_area).collect::<Vec<f32>>();

        let mut prev = 0.0;
        let mut cdf = vec![];
        for a in normalized_areas.iter() {
            let current = prev + a;
            cdf.push(current);
            prev = current;
        }
        info!("{:?}", cdf);

        let mut particles = vec![];
        let mut rng = rand::thread_rng();
        for _ in 0..NUM_PARTICLES {
            let idx = sample_cdf(rng.gen(), &cdf);
            let sampled_tri = &tris[idx as usize];
            // let sampled_tri = &tris[rng.gen_range(0..tris.len())];
            let p = uniform_sample_tri(rng.gen(), rng.gen(), sampled_tri);
            particles.push(p);
        }

        // Despawn any old particles
        particle_q.for_each(|e| commands.entity(e).despawn());

        for p in particles {
            commands.spawn((
                Particle,
                PbrBundle {
                    mesh: mesh_assets.add(Mesh::from(shape::UVSphere {
                        radius: 0.02,
                        ..default()
                    })),
                    material: mat_assets.add(StandardMaterial {
                        base_color: Color::RED,
                        ..default()
                    }),
                    transform: Transform::from_translation(p),
                    ..default()
                },
            ));
        }
    }
}

// e is a uniform sample in [0,1]
fn sample_cdf(e: f32, cdf: &Vec<f32>) -> i32 {
    let len = cdf.len() as i32;
    let mut l = 0;
    let mut r = len - 1;
    while l <= r {
        let mid = l + (r - l);

        // if cdf[mid as usize] == e {
        //     return mid;
        // }

        if cdf[mid as usize] <= e {
            l = mid + 1;
        } else if cdf[mid as usize] > e {
            r = mid - 1;
        }
    }

    return l.min(len - 1);
}

// https://extremelearning.com.au/evenly-distributing-points-in-a-triangle/
fn uniform_sample_tri(e0: f32, e1: f32, t: &[Vec3; 3]) -> Vec3 {
    let a = t[1] - t[0];
    let b = t[2] - t[0];
    if e0 + e1 < 1.0 {
        return t[0] + e0 * a + e1 * b;
    } else {
        return t[0] + (1.0 - e0) * a + (1.0 - e1) * b;
    }
}
