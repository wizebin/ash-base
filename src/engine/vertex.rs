#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
}
