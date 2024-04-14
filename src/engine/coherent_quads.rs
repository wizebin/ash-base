use std::{cell::RefCell, fmt::{Debug, Formatter}, mem::{self, size_of_val}, os::raw::c_void, sync::{Arc, Mutex}};

use ash::{util::Align, vk, Device};

use super::{memory::find_memorytype_index, vertex::Vertex};

pub struct CoherentQuads {
    pub local_index_buffer_data: Vec<u32>,
    pub device_index_buffer: vk::Buffer,
    pub local_vertex_buffer_data: RefCell<Vec<Vertex>>,
    pub device_vertex_buffer: vk::Buffer,
    pub current_max_quad_quantity: u32,
    pub index_buffer_memory: vk::DeviceMemory,
    pub vertex_input_buffer_memory: vk::DeviceMemory,
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
     pub unsafe fn new(max_quad_quantity: u32, device: Arc<Mutex<Device>>, device_memory_properties: vk::PhysicalDeviceMemoryProperties) -> Self {
        let local_vertex_buffer_data: RefCell<Vec<Vertex>> = RefCell::new(Vec::with_capacity(max_quad_quantity as usize * 4));
        let local_index_buffer_data: Vec<u32> = Vec::with_capacity(max_quad_quantity as usize * 6);

        let byte_size_of_single_vertex = mem::size_of::<Vertex>() as u64;
        let byte_size_of_index_instance = 6 * mem::size_of::<u32>() as u64;
        let byte_size_of_quad_instance = 4 * byte_size_of_single_vertex;

        let index_buffer_info = vk::BufferCreateInfo {
            size: byte_size_of_index_instance * (max_quad_quantity as u64),
            usage: vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let vertex_input_buffer_info = vk::BufferCreateInfo {
            size: byte_size_of_quad_instance * (max_quad_quantity as u64),
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let locked_device = device.clone();
        let locked_device = locked_device.lock().unwrap();

        let device_index_buffer = locked_device.create_buffer(&index_buffer_info, None).unwrap();
        let device_vertex_buffer = locked_device.create_buffer(&vertex_input_buffer_info, None).unwrap();

        let index_buffer_memory_req = locked_device.get_buffer_memory_requirements(device_index_buffer);
        let index_buffer_memory_index = find_memorytype_index(
            &index_buffer_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the index buffer.");
        let index_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: index_buffer_memory_req.size,
            memory_type_index: index_buffer_memory_index,
            ..Default::default()
        };
        let index_buffer_memory = locked_device
            .allocate_memory(&index_allocate_info, None)
            .unwrap();

        let vertex_input_buffer_memory_req = locked_device
            .get_buffer_memory_requirements(device_vertex_buffer);
        let vertex_input_buffer_memory_index = find_memorytype_index(
            &vertex_input_buffer_memory_req,
            &device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the vertex buffer.");

        let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: vertex_input_buffer_memory_req.size,
            memory_type_index: vertex_input_buffer_memory_index,
            ..Default::default()
        };
        let vertex_input_buffer_memory = locked_device
            .allocate_memory(&vertex_buffer_allocate_info, None)
            .unwrap();

        locked_device
            .bind_buffer_memory(device_vertex_buffer, vertex_input_buffer_memory, 0)
            .unwrap();

        locked_device
            .bind_buffer_memory(device_index_buffer, index_buffer_memory, 0)
            .unwrap();

        Self {
            device: device.clone(),
            local_index_buffer_data,
            device_index_buffer,
            local_vertex_buffer_data,
            device_vertex_buffer,
            current_max_quad_quantity: max_quad_quantity,
            index_buffer_memory,
            vertex_input_buffer_memory,
        }
    }

    pub fn index_quantity(&self) -> usize {
        self.local_index_buffer_data.len()
    }

    pub fn quad_quantity(&self) -> usize {
        self.local_vertex_buffer_data.borrow().len() / self.vertex_buffer_instance_node_quantity()
    }

    pub fn index_buffer_instance_node_quantity(&self) -> usize {
        6
    }

    pub fn vertex_buffer_instance_node_quantity(&self) -> usize {
        4
    }

    pub fn add_quad(&mut self, vertices: [Vertex; 4]) {
        let mut borrow_data = self.local_vertex_buffer_data.borrow_mut();

        borrow_data.extend_from_slice(&vertices);
        let index_offset = borrow_data.len() as u32 - 4;
        self.local_index_buffer_data.extend_from_slice(&[
            index_offset,
            index_offset + 1,
            index_offset + 2,
            index_offset + 2,
            index_offset + 3,
            index_offset,
        ]);
    }

    pub fn modify_quad(&self, index: usize, vertices: [Vertex; 4]) {
        let index_offset = index as u32 * 4;
        let mut borrow_data = self.local_vertex_buffer_data.borrow_mut();
        borrow_data[index_offset as usize..(index_offset + 4) as usize]
            .copy_from_slice(&vertices);
    }

    pub fn get_quad(&self, index: usize) -> [Vertex; 4] {
        let index_offset = index as u32 * 4;
        let borrow_data = self.local_vertex_buffer_data.borrow();
        [
            borrow_data[index_offset as usize],
            borrow_data[(index_offset + 1) as usize],
            borrow_data[(index_offset + 2) as usize],
            borrow_data[(index_offset + 3) as usize],
        ]
    }

    pub fn remap_data(&self) {
        let device = self.device.as_ref();
        match device.lock() {
            Ok(device) => {
                unsafe {
                    let index_buffer_memory_req = device.get_buffer_memory_requirements(self.device_index_buffer);
                    let index_ptr: *mut c_void = device
                        .map_memory(
                            self.index_buffer_memory,
                            0,
                            index_buffer_memory_req.size,
                            vk::MemoryMapFlags::empty(),
                        )
                        .unwrap();
                    let mut index_slice = Align::new(
                        index_ptr,
                        mem::align_of::<u32>() as u64,
                        index_buffer_memory_req.size,
                    );
                    index_slice.copy_from_slice(&self.local_index_buffer_data);
                    device.unmap_memory(self.index_buffer_memory);

                    let vertex_input_buffer_memory_req = device
                        .get_buffer_memory_requirements(self.device_vertex_buffer);
                    let vert_ptr = device
                        .map_memory(
                            self.vertex_input_buffer_memory,
                            0,
                            vertex_input_buffer_memory_req.size,
                            vk::MemoryMapFlags::empty(),
                        )
                        .unwrap();
                    let mut slice = Align::new(
                        vert_ptr,
                        mem::align_of::<Vertex>() as u64,
                        vertex_input_buffer_memory_req.size,
                    );
                    slice.copy_from_slice(&self.local_vertex_buffer_data.borrow());
                    device.unmap_memory(self.vertex_input_buffer_memory);
                }
            }
            Err(_) => {
                panic!("Failed to lock device mutex");
            }
        }
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

                    device.free_memory(self.index_buffer_memory, None);
                    device.free_memory(self.vertex_input_buffer_memory, None);
                }
            }
            Err(_) => {
                panic!("Failed to lock device mutex");
            }
        }
    }
}
