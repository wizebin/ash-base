use super::vertex::Vertex;

pub fn rotate_quad_vertices(vertices: [Vertex; 4], rotation: f32) -> [Vertex; 4] {
    let center_x = (vertices[0].pos[0] + vertices[2].pos[0]) / 2.0;
    let center_y = (vertices[0].pos[1] + vertices[2].pos[1]) / 2.0;
    let sin = rotation.sin();
    let cos = rotation.cos();
    let mut new_vertices = vertices;
    for vertex in new_vertices.iter_mut() {
        let x = vertex.pos[0] - center_x;
        let y = vertex.pos[1] - center_y;
        vertex.pos[0] = x * cos - y * sin + center_x;
        vertex.pos[1] = x * sin + y * cos + center_y;
    }
    new_vertices
}

pub fn make_quad_vertices(x: f32, y: f32, width: f32, height: f32, rotation: f32) -> [Vertex; 4] {
    let unrotated = [
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
    ];

    if rotation != 0.0 {
        rotate_quad_vertices(unrotated, rotation)
    } else {
        unrotated
    }
}
