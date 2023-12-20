use bevy::{
    core::FrameCount,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
    render::{MainWorld, RenderApp},
};
use bevy_egui::{
    egui::{self, Color32, Frame, Layout, Margin, RichText},
    EguiContexts, EguiPlugin,
};
use std::sync::Mutex;

pub struct PulseDiagnosticsPlugin;

pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408518866908483);
pub const FRAME_COUNT: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010279);
pub const FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(73441630925388532774622109283091159699);

impl Plugin for PulseDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<DiagnosticsStore>()
            .init_resource::<IntermediateDiagnosticsStore>()
            .add_systems(
                Update,
                (add_measurements, merge_diagnostics, display_diagnostics).chain(),
            );

        app.sub_app_mut(RenderApp)
            .init_resource::<DiagnosticsStore>()
            .add_systems(ExtractSchedule, transfer_render_world_diagnostics);

        // Register frame time
        app.register_diagnostic(
            Diagnostic::new(FRAME_TIME, "frame_time", 20)
                .with_suffix("ms")
                .with_smoothing_factor(1.0),
        );
        // Register fps
        app.register_diagnostic(Diagnostic::new(FPS, "fps", 20).with_smoothing_factor(1.0));
        // Register frame count
        app.register_diagnostic(Diagnostic::new(FRAME_COUNT, "frame_count", 1));
    }
}

pub fn add_measurements(
    mut diagnostics: Diagnostics,
    time: Res<Time<Real>>,
    frame_count: Res<FrameCount>,
) {
    diagnostics.add_measurement(FRAME_COUNT, || frame_count.0 as f64);

    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }

    diagnostics.add_measurement(FRAME_TIME, || delta_seconds * 1000.0);

    diagnostics.add_measurement(FPS, || 1.0 / delta_seconds);
}

// Diagnostics transferred from the render app
#[derive(Resource, Default)]
pub struct IntermediateDiagnosticsStore(pub Mutex<Vec<Diagnostic>>);

fn transfer_render_world_diagnostics(
    main_world: Res<MainWorld>,
    diagnostics: Res<DiagnosticsStore>,
) {
    let intermediate = &mut *main_world
        .resource::<IntermediateDiagnosticsStore>()
        .0
        .lock()
        .unwrap();

    *intermediate = diagnostics.iter().map(|d| (*d).clone()).collect::<Vec<_>>();
}

fn merge_diagnostics(
    intermediate: Res<IntermediateDiagnosticsStore>,
    mut diagnostics: ResMut<DiagnosticsStore>,
) {
    for new in intermediate.0.lock().unwrap().iter() {
        diagnostics.add(new.clone());
    }
}

fn display_diagnostics(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    let frame = Frame {
        inner_margin: Margin::symmetric(2.0, 6.0),
        rounding: egui::Rounding::from(1.0),
        fill: Color32::from_rgba_premultiplied(0, 0, 0, 210),
        ..default()
    };
    egui::Window::new("Diagnostics")
        .frame(frame)
        .title_bar(false)
        .interactable(false)
        .movable(true)
        .show(contexts.ctx_mut(), |ui| {
            egui::Grid::new("my_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    for diagnostic in diagnostics.iter() {
                        if let (Some(value), Some(avg)) =
                            (diagnostic.smoothed(), diagnostic.average())
                        {
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!("{}:", diagnostic.name))
                                        .monospace()
                                        .strong()
                                        .size(12.0)
                                        .color(Color32::WHITE),
                                );
                            });

                            // ui.label(
                            //     RichText::new(format!(
                            //         "{value:.3}{suffix}",
                            //         suffix = diagnostic.suffix,
                            //     ))
                            //     .monospace()
                            //     .size(12.0)
                            //     .color(Color32::WHITE),
                            // );

                            if diagnostic.get_max_history_length() > 1 {
                                ui.label(
                                    RichText::new(format!(
                                        "{value:.3}{suffix} ({avg:.3}{suffix} avg) ",
                                        suffix = diagnostic.suffix,
                                    ))
                                    .monospace()
                                    .size(12.0)
                                    .color(Color32::WHITE),
                                );
                            } else {
                                ui.label(
                                    RichText::new(format!(
                                        "{value:.3}{suffix}",
                                        suffix = diagnostic.suffix,
                                    ))
                                    .monospace()
                                    .size(12.0)
                                    .color(Color32::WHITE),
                                );
                            }

                            ui.end_row();
                        }
                    }
                });
        });
}
