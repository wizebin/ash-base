use super::vertex::Vertex;

pub fn make_quad_vertices(x: f32, y: f32, width: f32, height: f32) -> [Vertex; 4] {
    [
        Vertex {
            pos: [x, y, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            pos: [x, y + height, 0.0, 1.0],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [x + width, y + height, 0.0, 1.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            pos: [x + width, y, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
    ]
}
