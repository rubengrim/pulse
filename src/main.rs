use bevy::{prelude::*, render::camera::CameraRenderGraph};
use pulse::{path_tracer::*, PulsePlugin, PULSE_GRAPH};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PulsePlugin, PulsePathTracerPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // commands.spawn(PulsePathTracerCameraBundle::default());
    commands.spawn((
        Camera3dBundle {
            camera_render_graph: CameraRenderGraph::new(PULSE_GRAPH),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        PulsePathTracer::default(),
    ));

    // commands.spawn((
    //     Camera2dBundle {
    //         camera: Camera {
    //             hdr: true,
    //             ..default()
    //         },
    //         ..default()
    //     },
    //     PulsePathTracer::default(),
    // ));
}
