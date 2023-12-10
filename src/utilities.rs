use bevy::{
    prelude::*,
    render::{
        render_resource::{
            encase::internal::WriteInto, Buffer, BufferDescriptor, BufferUsages,
            CommandEncoderDescriptor, ShaderSize, ShaderType, StorageBuffer, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
    },
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

pub fn create_storage_buffer<T: ShaderSize + WriteInto>(
    vec: Vec<T>,
    label: Option<&str>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) -> StorageBuffer<Vec<T>> {
    let mut buffer = StorageBuffer::from(vec);
    if label.is_some() {
        buffer.set_label(label);
    }
    buffer.set_label(label);
    buffer.add_usages(BufferUsages::STORAGE);
    buffer.write_buffer(render_device, render_queue);
    buffer
}

pub fn transform(m: Mat4, v: Vec3) -> Vec3 {
    let homogeneous = m.mul_vec4(Vec4::new(v.x, v.y, v.z, 1.0));
    homogeneous.xyz() / homogeneous.w
}

pub fn transform_dir(m: Mat4, v: Vec3) -> Vec3 {
    m.mul_vec4(Vec4::new(v.x, v.y, v.z, 0.0)).xyz()
}
