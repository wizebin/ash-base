use ash::vk;

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }

    pub fn extent(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }

    pub fn extent2d(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.width,
            height: self.height,
        }
    }
}
