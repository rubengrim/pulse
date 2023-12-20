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

pub fn transform_position(v: Vec3, m: Mat4) -> Vec3 {
    let homogeneous = m.mul_vec4(Vec4::new(v.x, v.y, v.z, 1.0));
    homogeneous.xyz() / homogeneous.w
}

pub fn transform_direction(v: Vec3, m: Mat4) -> Vec3 {
    m.mul_vec4(Vec4::new(v.x, v.y, v.z, 0.0)).xyz()
}

pub fn swap<T: Clone>(data: &mut [T], i0: usize, i1: usize) {
    // TODO: Error handling
    let val0 = data[i0].clone();
    data[i0] = data[i1].clone();
    data[i1] = val0;
}
