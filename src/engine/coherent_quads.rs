use std::{fmt::{Debug, Formatter}, mem::{self, size_of_val}, sync::{Arc, Mutex}};

use ash::{vk, Device};

use super::vertex::Vertex;

pub struct CoherentQuads {
    pub local_index_buffer_data: Vec<u32>,
    pub device_index_buffer: vk::Buffer,
    pub local_vertex_buffer_data: Vec<Vertex>,
    pub device_vertex_buffer: vk::Buffer,
    pub current_max_quad_quantity: u32,
    pub device: Arc<Mutex<Device>>,
}

impl Debug for CoherentQuads {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoherentQuads")
            .field("local_index_buffer_data", &self.local_index_buffer_data)
            .field("device_index_buffer", &self.device_index_buffer)
            .field("local_vertex_buffer_data", &self.local_vertex_buffer_data)
            .field("device_vertex_buffer", &self.device_vertex_buffer)
            .field("current_max_quad_quantity", &self.current_max_quad_quantity)
            .finish()
    }
}

impl CoherentQuads {
    pub fn new(max_quad_quantity: u32, device: Arc<Mutex<Device>>) -> Self {
        let local_vertex_buffer_data: Vec<Vertex> = Vec::with_capacity(max_quad_quantity as usize * 4);
        let local_index_buffer_data: Vec<u32> = Vec::with_capacity(max_quad_quantity as usize * 6);

        let index_buffer_info = vk::BufferCreateInfo {
            size: size_of_val(&local_index_buffer_data) as u64,
            usage: vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let byte_size_of_single_vertex = mem::size_of::<Vertex>() as u64;

        let vertex_input_buffer_info = vk::BufferCreateInfo {
            size: byte_size_of_single_vertex * (max_quad_quantity as u64),
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let (device_index_buffer, device_vertex_buffer) = match device.clone().lock() {
            Ok(device) => {
                unsafe {
                    (
                        device.create_buffer(&index_buffer_info, None).unwrap(),
                        device.create_buffer(&vertex_input_buffer_info, None).unwrap()
                    )
                }
            }
            Err(_) => {
                panic!("Failed to lock device mutex");
            }
        };

        Self {
            device: device.clone(),
            local_index_buffer_data,
            device_index_buffer,
            local_vertex_buffer_data,
            device_vertex_buffer,
            current_max_quad_quantity: max_quad_quantity,
        }
    }

    pub fn add_quad(&mut self, vertices: [Vertex; 4]) {
        self.local_vertex_buffer_data.extend_from_slice(&vertices);
        let index_offset = self.local_vertex_buffer_data.len() as u32 - 4;
        self.local_index_buffer_data.extend_from_slice(&[
            index_offset,
            index_offset + 1,
            index_offset + 2,
            index_offset + 2,
            index_offset + 3,
            index_offset,
        ]);
    }
}

impl Drop for CoherentQuads {
    fn drop(&mut self) {
        let device = self.device.as_ref();
        match device.lock() {
            Ok(device) => {
                unsafe {
                    device.destroy_buffer(self.device_index_buffer, None);
                    device.destroy_buffer(self.device_vertex_buffer, None);
                }
            }
            Err(_) => {
                panic!("Failed to lock device mutex");
            }
        }
    }
}
