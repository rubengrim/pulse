use std::sync::Mutex;

use bevy::{
    prelude::*,
    render::{MainWorld, RenderApp},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub struct PulseProfilingPlugin;

impl Plugin for PulseProfilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<ProfilingEntries>()
            .add_systems(Update, draw_ui);

        app.sub_app_mut(RenderApp)
            .init_resource::<RenderWorldProfilingEntries>()
            .add_systems(ExtractSchedule, send_back_profiling_entries);
    }
}

// Stores Vec<(label, value)>
#[derive(Resource, Default)]
pub struct ProfilingEntries(pub Mutex<Vec<(String, String)>>);

impl ProfilingEntries {
    pub fn add_entry(&mut self, label: String, value: String) {
        let entries = &mut *self.0.lock().unwrap();
        entries.push((label, value));
    }
}

// Stores Vec<(label, value)>
#[derive(Resource, Default)]
pub struct RenderWorldProfilingEntries(pub Vec<(String, String)>);

impl RenderWorldProfilingEntries {
    pub fn add_entry(&mut self, label: String, value: String) {
        self.0.push((label, value));
    }
}

// Copies over entries to corresponding resource in the main world.
fn send_back_profiling_entries(
    main_world: ResMut<MainWorld>,
    mut render_world_entries: ResMut<RenderWorldProfilingEntries>,
) {
    let entries = &mut *main_world.resource::<ProfilingEntries>().0.lock().unwrap();
    entries.append(&mut render_world_entries.0);
    render_world_entries.0 = vec![];
}

fn draw_ui(mut contexts: EguiContexts, entries: Res<ProfilingEntries>) {
    egui::Window::new("Statistics").show(contexts.ctx_mut(), |ui| {
        let entries = &mut *entries.0.lock().unwrap();
        for entry in entries.iter() {
            ui.monospace(format!("{}: {}", entry.0, entry.1));
        }
        *entries = vec![];
    });
}
