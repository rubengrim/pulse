use bevy::prelude::*;

pub mod node;
pub use node::*;

pub mod pipeline;
pub use pipeline::*;

pub const PULSE_UPSCALING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(338718014948591213280805766994766947065);
