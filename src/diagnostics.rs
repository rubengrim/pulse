use bevy::{
    diagnostic::{Diagnostic, DiagnosticsStore, MAX_DIAGNOSTIC_NAME_WIDTH},
    prelude::*,
    render::{MainWorld, RenderApp},
};
use bevy_egui::{
    egui::{self, epaint::Shadow, Color32, Frame, Layout, Margin, RichText, Stroke, Style},
    EguiContexts, EguiPlugin,
};
use std::sync::Mutex;

pub struct PulseDiagnosticsPlugin;

impl Plugin for PulseDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<DiagnosticsStore>()
            .init_resource::<IntermediateDiagnosticsStore>()
            .add_systems(Update, (merge_diagnostics, display_diagnostics).chain());

        app.sub_app_mut(RenderApp)
            .init_resource::<DiagnosticsStore>()
            .add_systems(ExtractSchedule, transfer_render_world_diagnostics);
    }
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
                        if let (Some(value), Some(_avg)) =
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

                            ui.label(
                                RichText::new(format!(
                                    "{value:.3}{suffix}",
                                    suffix = diagnostic.suffix,
                                ))
                                .monospace()
                                .size(12.0)
                                .color(Color32::WHITE),
                            );

                            // if diagnostic.get_max_history_length() > 1 {
                            //     ui.label(
                            //         RichText::new(format!(
                            //             "{value:.3}{suffix} [{avg:.3}{suffix} moving average]",
                            //             suffix = diagnostic.suffix,
                            //         ))
                            //         .monospace()
                            //         .size(12.0)
                            //         .color(Color32::WHITE),
                            //     );
                            // } else {
                            //     ui.label(
                            //         RichText::new(format!(
                            //             "{value:.3}{suffix}",
                            //             suffix = diagnostic.suffix,
                            //         ))
                            //         .monospace()
                            //         .size(12.0)
                            //         .color(Color32::WHITE),
                            //     );
                            // }

                            ui.end_row();
                        }
                    }
                });
        });
}
