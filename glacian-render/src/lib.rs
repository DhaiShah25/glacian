pub mod terrain;

#[cfg(feature = "vulkan")]
mod vk_render;
#[cfg(feature = "vulkan")]
pub use vk_render::VkRenderer as Renderer;

#[cfg(all(
    feature = "vulkan",
    not(any(target_os = "windows", target_os = "linux"))
))]
compile_error!("The 'vulkan' feature is only supported on Windows and Linux.");

#[cfg(feature = "wgpu")]
mod wgpu_render;
#[cfg(feature = "wgpu")]
pub use wgpu_render::WgpuRenderer as Renderer;

#[cfg(not(any(feature = "vulkan", feature = "wgpu")))]
compile_error!("Vulkan or wgpu has to be enabled");

// pub trait VoxelTerrain {
//     /// Returns the raw bytes of a texture atlas
//     fn get_texture_atlas_bytes() -> &'static [u8];
//
//     /// Gets the voxel type (e.g., solid or air) and its texture index.
//     /// The returned tuple is (is_solid, texture_index).
//     fn get_type(&self, x: i32, y: i32, z: i32) -> (bool, u32);
// }
//
// // An enum to represent the level of detail.
// // Each variant corresponds to a downsampling factor.
// pub enum LoD {
//     Full,
//     Half,
//     Quarter,
//     Eighth,
//     Sixteenth,
// }
//
// impl LoD {
//     fn get_step_size(&self) -> usize {
//         match self {
//             LoD::Full => 1,
//             LoD::Half => 2,
//             LoD::Quarter => 4,
//             LoD::Eighth => 8,
//             LoD::Sixteenth => 16,
//         }
//     }
// }
//
// enum TerrainNormal {
//     Up,
//     Down,
//     Left,
//     Right,
//     Front,
//     Back,
// }
//
// struct TerrainVertex {
//     normal: TerrainNormal,
//     position: glam::UVec3,
// }
//
// struct Chunk {
//     vertices: Vec<TerrainVertex>,
//     indices: Vec<u16>,
// }
//
// impl Chunk {
//     fn new<T: VoxelTerrain>(chunk: glam::IVec3, terrain: T) -> Self {
//         let mut vertices = Vec::new();
//         let indices = Vec::new();
//
//         const CHUNK_SIZE: i32 = 64;
//
//         for i in 0..CHUNK_SIZE {
//             for j in 0..CHUNK_SIZE {
//                 for k in 0..CHUNK_SIZE {
//                     let coord = glam::ivec3(i, j, k) + chunk * CHUNK_SIZE;
//                     let data = terrain.get_type(coord.x, coord.y, coord.z);
//                     vertices.push(data.0);
//                 }
//             }
//         }
//
//         Self {
//             vertices: Vec::new(),
//             indices,
//         }
//     }
// }
