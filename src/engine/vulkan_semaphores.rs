use std::sync::{Arc, Mutex};

use ash::{Device, vk};

pub fn create_semaphores(device: Arc<Mutex<Device>>, semaphore_count: usize) -> Vec<vk::Semaphore> {
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();

    let semaphore_create_info = vk::SemaphoreCreateInfo::default();

    (0..semaphore_count)
        .map(|_| unsafe {locked_device
            .create_semaphore(&semaphore_create_info, None)
            .unwrap() }
        ).collect()
}
