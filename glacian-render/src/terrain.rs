use modular_bitfield::prelude::*;

#[derive(Specifier)]
#[bits = 3]
pub enum Normal {
    Up,
    Down,
    Left,
    Right,
    Front,
    Back,
}

#[bitfield]
pub struct Vertex {
    x: B5,
    y: B5,
    z: B5,
    texture_x: B5,
    texture_y: B5,
    normal: Normal,
    __: B4,
}

// Create a world using the noise crate and then use
// the binary-greedy-meshing Mesher to generate quads
// Then create vertexes
