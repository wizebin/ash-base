use std::collections::HashMap;

use super::vulkan_image::VulkanImage;

pub struct ImageManager {
    pub images: HashMap<&'static str, VulkanImage>,
}

impl ImageManager {
    pub fn new() -> Self {
        Self {
            images: HashMap::new(),
        }
    }

    pub fn get_image(&self, name: &'static str) -> &VulkanImage {
        self.images.get(name).unwrap()
    }

    pub fn add_image(&mut self, name: &'static str, image: VulkanImage) {
        self.images.insert(name, image);
    }

    pub fn clear(&mut self) {
        self.images.clear();
    }
}
