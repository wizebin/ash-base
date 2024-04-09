use std::sync::{Arc, Mutex};

use ash::{Device, vk};

pub fn create_standard_fences(device: Arc<Mutex<Device>>, fence_count: u32) -> Vec<vk::Fence> {
    let mut fences = Vec::with_capacity(fence_count as usize);
    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let locked_device = device.clone();
    let locked_device = locked_device.lock().unwrap();
    for _ in 0..fence_count {
        let fence = unsafe { locked_device.create_fence(&fence_info, None).unwrap() };
        fences.push(fence);
    }

    fences
}
