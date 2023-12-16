use bevy::{
    diagnostic::{Diagnostic, DiagnosticsStore},
    prelude::*,
    render::{MainWorld, RenderApp},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
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

    *intermediate = diagnostics.iter().map(|d| d.clone()).collect::<Vec<_>>();
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
    egui::Window::new("Diagnostics").show(contexts.ctx_mut(), |ui| {
        for diagnostic in diagnostics.iter() {
            if let Some(value) = diagnostic.smoothed() {
                ui.monospace(format!(
                    "{}: {}{}",
                    diagnostic.name, value, diagnostic.suffix
                ));
            }
        }
    });
}
