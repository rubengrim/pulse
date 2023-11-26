use bevy::render::{
    render_resource::{
        encase::internal::WriteInto, Buffer, BufferDescriptor, BufferUsages,
        CommandEncoderDescriptor, ShaderType, UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
};

pub fn create_uniform_buffer<T: ShaderType + WriteInto>(
    s: T,
    label: Option<&str>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) -> UniformBuffer<T> {
    let mut buffer = UniformBuffer::from(s);
    if label.is_some() {
        buffer.set_label(label);
    }
    buffer.write_buffer(render_device, render_queue);
    buffer
}
