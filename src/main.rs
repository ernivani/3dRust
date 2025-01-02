mod gl_utils;
mod math;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use math::{Mat4, Vec3};
use std::thread;
use std::time::Duration;
use std::fs;
use std::collections::HashMap;
use noise::{NoiseFn, Perlin};

type Vertex = [f32; 8];  // x, y, z, s, t, position, textureIndex, textSize
type TriIndexes = [u32; 3];

const CHUNK_SIZE: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct BlockPosition {
    x: usize,
    y: usize,
    z: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum BlockType {
    Air,
    Grass,
    Dirt,
    Stone,
    Water,
}

struct Chunk {
    position: (i32, i32, i32),  // Chunk position in world space
    blocks: Vec<Vec<Vec<BlockType>>>,
    visible_blocks: HashMap<BlockPosition, BlockType>,
    vertices: Vec<Vertex>,
    indices: Vec<TriIndexes>,
    vertex_count: u32,
}

impl Chunk {
    fn new(position: (i32, i32, i32)) -> Self {
        let mut chunk = Self {
            position,
            blocks: vec![vec![vec![BlockType::Air; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
            visible_blocks: HashMap::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_count: 0,
        };
        chunk.generate_terrain();
        chunk
    }

    fn generate_terrain(&mut self) {
        // Create noise generators
        let terrain_noise = Perlin::new(42);  // Base terrain height
        let detail_noise = Perlin::new(123);  // Additional detail
        let cave_noise = Perlin::new(666);    // Cave system

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                // Convert local coordinates to global coordinates
                let world_x = self.position.0 * CHUNK_SIZE as i32 + x as i32;
                let world_z = self.position.2 * CHUNK_SIZE as i32 + z as i32;
                
                // Generate base terrain height
                let nx = world_x as f64 * 0.02;
                let nz = world_z as f64 * 0.02;
                
                // Combine different noise layers for more interesting terrain
                let base_height = terrain_noise.get([nx, nz]) * 32.0 + 64.0;  // Base terrain
                let detail = detail_noise.get([nx * 4.0, nz * 4.0]) * 8.0;    // Small details
                let height = (base_height + detail) as i32;

                for y in 0..CHUNK_SIZE {
                    let world_y = self.position.1 * CHUNK_SIZE as i32 + y as i32;
                    
                    // Cave generation
                    let cave_value = cave_noise.get([
                        world_x as f64 * 0.05,
                        world_y as f64 * 0.05,
                        world_z as f64 * 0.05
                    ]);

                    // Determine block type based on height and noise values
                    if world_y < height {
                        // Cave generation
                        if cave_value > 0.6 {
                            self.blocks[x][y][z] = BlockType::Air;
                        } else {
                            // Normal terrain
                            if world_y == height - 1 {
                                self.blocks[x][y][z] = BlockType::Grass;
                            } else if world_y > height - 4 {
                                self.blocks[x][y][z] = BlockType::Dirt;
                            } else {
                                self.blocks[x][y][z] = BlockType::Stone;
                            }
                        }
                    } else if world_y < 60 { // Water level
                        self.blocks[x][y][z] = BlockType::Water;
                    } else {
                        self.blocks[x][y][z] = BlockType::Air;
                    }
                }
            }
        }
    }

    fn update(&mut self, world: &World) {
        // Clear previous data
        self.visible_blocks.clear();
        self.vertices.clear();
        self.indices.clear();
        self.vertex_count = 0;

        // Identify visible blocks
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let block_type = self.blocks[x][y][z];
                    if block_type != BlockType::Air {
                        // Convert to world coordinates
                        let world_x = self.position.0 * CHUNK_SIZE as i32 + x as i32;
                        let world_y = self.position.1 * CHUNK_SIZE as i32 + y as i32;
                        let world_z = self.position.2 * CHUNK_SIZE as i32 + z as i32;

                        // Check if any face is visible using world coordinates
                        if should_render_face(world, world_x, world_y, world_z, "front") ||
                           should_render_face(world, world_x, world_y, world_z, "back") ||
                           should_render_face(world, world_x, world_y, world_z, "top") ||
                           should_render_face(world, world_x, world_y, world_z, "bottom") ||
                           should_render_face(world, world_x, world_y, world_z, "right") ||
                           should_render_face(world, world_x, world_y, world_z, "left") {
                            self.visible_blocks.insert(BlockPosition { x, y, z }, block_type);
                        }
                    }
                }
            }
        }

        // Generate vertices and indices for visible blocks
        for (&block_pos, &block_type) in &self.visible_blocks {
            let world_x = (self.position.0 * CHUNK_SIZE as i32) as f32 + block_pos.x as f32;
            let world_y = (self.position.1 * CHUNK_SIZE as i32) as f32 + block_pos.y as f32;
            let world_z = (self.position.2 * CHUNK_SIZE as i32) as f32 + block_pos.z as f32;

            let cube_vertices = generate_cube_vertices(
                world_x,
                world_y,
                world_z,
                block_type,
                world,
                world_x as i32,
                world_y as i32,
                world_z as i32
            );
            
            if !cube_vertices.is_empty() {
                let cube_indices = generate_indices_for_vertices(self.vertex_count, cube_vertices.len() as u32);
                self.vertices.extend_from_slice(&cube_vertices);
                self.indices.extend_from_slice(&cube_indices);
                self.vertex_count += cube_vertices.len() as u32;
            }
        }
    }
}

struct World {
    chunks: HashMap<(i32, i32, i32), Chunk>,
}

impl World {
    fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> BlockType {
        // Determine which chunk these coords belong to
        let chunk_x = world_x.div_euclid(CHUNK_SIZE as i32);
        let chunk_y = world_y.div_euclid(CHUNK_SIZE as i32);
        let chunk_z = world_z.div_euclid(CHUNK_SIZE as i32);

        // Find that chunk
        if let Some(chunk) = self.chunks.get(&(chunk_x, chunk_y, chunk_z)) {
            // Convert to local coords within chunk
            let lx = (world_x.rem_euclid(CHUNK_SIZE as i32)) as usize;
            let ly = (world_y.rem_euclid(CHUNK_SIZE as i32)) as usize;
            let lz = (world_z.rem_euclid(CHUNK_SIZE as i32)) as usize;

            chunk.blocks[lx][ly][lz]
        } else {
            BlockType::Air
        }
    }

    fn add_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.position, chunk);
    }
}

// Function to check if a face should be rendered based on adjacent blocks
fn should_render_face(world: &World, world_x: i32, world_y: i32, world_z: i32, face: &str) -> bool {
    let check_pos = match face {
        "front" => (world_x, world_y, world_z + 1),
        "back" => (world_x, world_y, world_z - 1),
        "top" => (world_x, world_y + 1, world_z),
        "bottom" => (world_x, world_y - 1, world_z),
        "right" => (world_x + 1, world_y, world_z),
        "left" => (world_x - 1, world_y, world_z),
        _ => return true,
    };
    
    // Special case for water: always render faces between water blocks
    let current_block = world.get_block(world_x, world_y, world_z);
    let neighbor_block = world.get_block(check_pos.0, check_pos.1, check_pos.2);
    
    match current_block {
        BlockType::Water => {
            // For water, only render faces between water and non-water blocks
            // or if the neighbor is air
            neighbor_block == BlockType::Air || neighbor_block != BlockType::Water
        },
        _ => {
            // For solid blocks, render face if neighbor is air or water
            neighbor_block == BlockType::Air || neighbor_block == BlockType::Water
        }
    }
}

// Function to generate vertices for a cube at a specific position
fn generate_cube_vertices(x: f32, y: f32, z: f32, block_type: BlockType, world: &World, 
    world_x: i32, world_y: i32, world_z: i32) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    
    match block_type {
        BlockType::Air => Vec::new(),
        BlockType::Grass => {
            // Front face
            if should_render_face(world, world_x, world_y, world_z, "front") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 0.0, 1.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 1.0, 1.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 2.0, 1.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 3.0, 1.0, 1.0],
                ]);
            }
            
            // Back face (grass_block_side)
            if should_render_face(world, world_x, world_y, world_z, "back") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 4.0, 1.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 5.0, 1.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 6.0, 1.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 7.0, 1.0, 1.0],
                ]);
            }
            
            // Top face (grass_block_top)
            if should_render_face(world, world_x, world_y, world_z, "top") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 8.0, 0.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 9.0, 0.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 1.0, 10.0, 0.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 1.0, 11.0, 0.0, 1.0],
                ]);
            }
            
            // Bottom face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "bottom") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  0.0, 0.0, 12.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  1.0, 0.0, 13.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 14.0, 2.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 15.0, 2.0, 1.0],
                ]);
            }
            
            // Right face (grass_block_side)
            if should_render_face(world, world_x, world_y, world_z, "right") {
                vertices.extend_from_slice(&[
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 16.0, 1.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 17.0, 1.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 18.0, 1.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 19.0, 1.0, 1.0],
                ]);
            }
            
            // Left face (grass_block_side)
            if should_render_face(world, world_x, world_y, world_z, "left") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 20.0, 1.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 21.0, 1.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 22.0, 1.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 23.0, 1.0, 1.0],
                ]);
            }
            vertices
        },
        BlockType::Dirt => {
            // Front face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "front") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 0.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 1.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 2.0, 2.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 3.0, 2.0, 1.0],
                ]);
            }
            
            // Back face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "back") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 4.0, 2.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 5.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 6.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 7.0, 2.0, 1.0],
                ]);
            }
            
            // Top face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "top") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 8.0, 2.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 9.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 1.0, 10.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 1.0, 11.0, 2.0, 1.0],
                ]);
            }
            
            // Bottom face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "bottom") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  0.0, 0.0, 12.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  1.0, 0.0, 13.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 14.0, 2.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 15.0, 2.0, 1.0],
                ]);
            }
            
            // Right face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "right") {
                vertices.extend_from_slice(&[
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 16.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 17.0, 2.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 18.0, 2.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 19.0, 2.0, 1.0],
                ]);
            }
            
            // Left face (dirt)
            if should_render_face(world, world_x, world_y, world_z, "left") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 20.0, 2.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 21.0, 2.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 22.0, 2.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 23.0, 2.0, 1.0],
                ]);
            }
            vertices
        },
        BlockType::Stone => {
            // Front face
            if should_render_face(world, world_x, world_y, world_z, "front") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 0.0, 3.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 1.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 2.0, 3.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 3.0, 3.0, 1.0],
                ]);
            }
            
            // Back face
            if should_render_face(world, world_x, world_y, world_z, "back") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 4.0, 3.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 5.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 6.0, 3.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 7.0, 3.0, 1.0],
                ]);
            }
            
            // Top face
            if should_render_face(world, world_x, world_y, world_z, "top") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 8.0, 3.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 9.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 1.0, 10.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 1.0, 11.0, 3.0, 1.0],
                ]);
            }
            
            // Bottom face
            if should_render_face(world, world_x, world_y, world_z, "bottom") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  0.0, 0.0, 12.0, 3.0, 1.0],
                    [x + 0.5, y - 0.5, z - 0.5,  1.0, 0.0, 13.0, 3.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 14.0, 3.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 15.0, 3.0, 1.0],
                ]);
            }
            
            // Right face
            if should_render_face(world, world_x, world_y, world_z, "right") {
                vertices.extend_from_slice(&[
                    [x + 0.5, y - 0.5, z - 0.5,  0.0, 1.0, 16.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z - 0.5,  0.0, 0.0, 17.0, 3.0, 1.0],
                    [x + 0.5, y + 0.5, z + 0.5,  1.0, 0.0, 18.0, 3.0, 1.0],
                    [x + 0.5, y - 0.5, z + 0.5,  1.0, 1.0, 19.0, 3.0, 1.0],
                ]);
            }
            
            // Left face
            if should_render_face(world, world_x, world_y, world_z, "left") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y - 0.5, z - 0.5,  1.0, 1.0, 20.0, 3.0, 1.0],
                    [x - 0.5, y - 0.5, z + 0.5,  0.0, 1.0, 21.0, 3.0, 1.0],
                    [x - 0.5, y + 0.5, z + 0.5,  0.0, 0.0, 22.0, 3.0, 1.0],
                    [x - 0.5, y + 0.5, z - 0.5,  1.0, 0.0, 23.0, 3.0, 1.0],
                ]);
            }
            vertices
        },
        BlockType::Water => {
            // Only render top face of water with transparency
            if should_render_face(world, world_x, world_y, world_z, "top") {
                vertices.extend_from_slice(&[
                    [x - 0.5, y + 0.4, z - 0.5,  0.0, 0.0, 8.0, 4.0, 1.0],  // Slightly lower than full block
                    [x - 0.5, y + 0.4, z + 0.5,  1.0, 0.0, 9.0, 4.0, 1.0],
                    [x + 0.5, y + 0.4, z + 0.5,  1.0, 1.0, 10.0, 4.0, 1.0],
                    [x + 0.5, y + 0.4, z - 0.5,  0.0, 1.0, 11.0, 4.0, 1.0],
                ]);
            }
            vertices
        },
    }
}

// Function to generate indices for vertices
fn generate_indices_for_vertices(vertex_offset: u32, vertex_count: u32) -> Vec<TriIndexes> {
    let mut indices = Vec::new();
    for i in (0..vertex_count).step_by(4) {
        indices.push([
            vertex_offset + i,
            vertex_offset + i + 1,
            vertex_offset + i + 2,
        ]);
        indices.push([
            vertex_offset + i + 2,
            vertex_offset + i + 3,
            vertex_offset + i,
        ]);
    }
    indices
}

// Add camera struct
struct Camera {
    position: Vec3,
    front: Vec3,
    up: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 100.0, 0.0),  // Moved back and up to see the chunks
            front: Vec3::new(0.0, -0.3, -1.0),      // Looking slightly down
            up: Vec3::new(0.0, 1.0, 0.0),
            yaw: -90.0,
            pitch: -15.0,
        }
    }

    fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.position + self.front, self.up)
    }

    fn update_camera_vectors(&mut self) {
        let front = Vec3::new(
            self.yaw.to_radians().cos() * self.pitch.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.yaw.to_radians().sin() * self.pitch.to_radians().cos()
        );
        self.front = front.normalize();
    }
}

fn load_shader(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read shader file: {}", path))
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_flags().debug().set();

    let window = video_subsystem
        .window("OpenGL Window", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();
    
    let _gl_context = window.gl_create_context().unwrap();
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

    unsafe {
        gl::Enable(gl::DEBUG_OUTPUT);
        gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
    }

    // Load and create shader program
    let vertex_shader = load_shader("src/assets/shaders/block.vert");
    let fragment_shader = load_shader("src/assets/shaders/block.frag");
    let shader_program = gl_utils::ShaderProgram::from_vert_frag(&vertex_shader, &fragment_shader)
        .expect("Failed to create shader program");

    // Create and set up VAO, VBO, and EBO
    let vao = gl_utils::VertexArray::new().expect("Failed to create VAO");
    let vbo = gl_utils::Buffer::new().expect("Failed to create VBO");
    let ebo = gl_utils::Buffer::new().expect("Failed to create EBO");
    
    vao.bind();
    
    // Generate chunks data
    let mut world = World::new();

    // Create a larger world (8x8x8 chunks)
    for chunk_x in -8..8 {
        for chunk_y in 0..8 {
            for chunk_z in -8..8 {
                let chunk = Chunk::new((chunk_x, chunk_y, chunk_z));
                world.add_chunk(chunk);
            }
        }
    }
    
    // Update all chunks after they're all created
    let mut all_vertices: Vec<Vertex> = Vec::new();
    let mut all_indices: Vec<TriIndexes> = Vec::new();

    // First pass: update all chunks
    let positions = world.chunks.keys().cloned().collect::<Vec<_>>();
    for pos in positions {
        // Get the blocks data
        let blocks = world.chunks[&pos].blocks.clone();
        
        // Remove the chunk temporarily
        let mut chunk = world.chunks.remove(&pos).unwrap();
        
        // Update the chunk
        chunk.blocks = blocks;
        chunk.update(&world);
        
        // Put the chunk back
        world.chunks.insert(pos, chunk);
    }

    // Second pass: collect vertices and indices
    for pos in world.chunks.keys().cloned().collect::<Vec<_>>() {
        if let Some(chunk) = world.chunks.get(&pos) {
            let vertex_offset = all_vertices.len() as u32;
            all_vertices.extend_from_slice(&chunk.vertices);
            
            for tri in &chunk.indices {
                all_indices.push([
                    tri[0] + vertex_offset,
                    tri[1] + vertex_offset,
                    tri[2] + vertex_offset,
                ]);
            }
        }
    }

    // Set up vertex buffer with all chunks data
    vbo.bind(gl_utils::BufferType::Array);
    gl_utils::buffer_data(
        gl_utils::BufferType::Array,
        bytemuck::cast_slice(&all_vertices),
        gl::STATIC_DRAW,
    );

    // Set up element buffer with all chunks indices
    ebo.bind(gl_utils::BufferType::ElementArray);
    gl_utils::buffer_data(
        gl_utils::BufferType::ElementArray,
        bytemuck::cast_slice(&all_indices),
        gl::STATIC_DRAW,
    );

    unsafe {
        // Position attribute
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            8 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            std::ptr::null(),
        );
        gl::EnableVertexAttribArray(0);

        // Texture coordinate attribute
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            8 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (3 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(1);

        // Position attribute
        gl::VertexAttribPointer(
            2,
            1,
            gl::FLOAT,
            gl::FALSE,
            8 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (5 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(2);

        // TextureIndex attribute
        gl::VertexAttribPointer(
            3,
            1,
            gl::FLOAT,
            gl::FALSE,
            8 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (6 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(3);

        // TextSize attribute
        gl::VertexAttribPointer(
            4,
            1,
            gl::FLOAT,
            gl::FALSE,
            8 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (7 * std::mem::size_of::<f32>()) as *const _,
        );
        gl::EnableVertexAttribArray(4);
    }

    // Load textures
    let grass_top_texture = gl_utils::load_texture("src/assets/textures/block/grass_block_top.png");
    let grass_side_texture = gl_utils::load_texture("src/assets/textures/block/grass_block_side.png");
    let grass_side_overlay_texture = gl_utils::load_texture("src/assets/textures/block/grass_block_side_overlay.png");
    let dirt_texture = gl_utils::load_texture("src/assets/textures/block/dirt.png");
    let colormap_texture = gl_utils::load_texture("src/assets/textures/colormap/grass.png");
    let stone_texture = gl_utils::load_texture("src/assets/textures/block/stone.png");
    let water_texture = gl_utils::load_texture("src/assets/textures/block/water_still.png");

    shader_program.use_program();

    // Set texture uniforms
    unsafe {
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"grassTopTexture\0".as_ptr() as *const i8), 0);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"grassSideTexture\0".as_ptr() as *const i8), 1);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"dirtTexture\0".as_ptr() as *const i8), 2);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"colormapTexture\0".as_ptr() as *const i8), 3);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"grassSideOverlayTexture\0".as_ptr() as *const i8), 4);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"stoneTexture\0".as_ptr() as *const i8), 5);
        gl::Uniform1i(gl::GetUniformLocation(shader_program.0, b"waterTexture\0".as_ptr() as *const i8), 6);
    }

    // Enable depth testing and blending for water transparency
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::Enable(gl::CULL_FACE);  // Enable face culling
        gl::CullFace(gl::BACK);     // Cull back faces
        gl::FrontFace(gl::CCW);     // Front faces are counter-clockwise
    }

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Initialize camera
    let mut camera = Camera::new();
    let projection = Mat4::perspective(45.0_f32.to_radians(), 800.0 / 600.0, 0.1, 1000.0);

    // Mouse handling setup
    let mouse = sdl_context.mouse();
    mouse.set_relative_mouse_mode(true);
    let mouse_sensitivity = 0.10;
    
    let timer = sdl_context.timer().unwrap();
    let mut last_frame_time = timer.ticks() as f32;
    let mut frame_count = 0;
    let mut last_fps_update = timer.ticks();
    let target_frame_time = 1000.0 / 60.0; // Target 60 FPS (in milliseconds)
    // Movement speed (units per second instead of per frame)
    let movement_speed = 10.5;

    'main_loop: loop {
        let current_frame_time = timer.ticks() as f32;
        let delta_time = (current_frame_time - last_frame_time) / 1000.0; // Convert to seconds
        last_frame_time = current_frame_time;

        // FPS Counter
        frame_count += 1;
        if current_frame_time - last_fps_update as f32 >= 1000.0 {
            println!("FPS: {}", frame_count);
            frame_count = 0;
            last_fps_update = current_frame_time as u32;
        }

        // Handle keyboard state
        let keyboard_state = event_pump.keyboard_state();
        
        // Camera movement with delta time
        let camera_speed = movement_speed * delta_time;
        let sprint = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LShift);
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::W) {
            camera.position = camera.position + camera.front * camera_speed * if sprint { 2.0 } else { 1.0 };
        }
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::S) {
            camera.position = camera.position - camera.front * camera_speed * if sprint { 2.0 } else { 1.0 };
        }
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::A) {
            let right = camera.front.cross(&camera.up).normalize();
            camera.position = camera.position - right * camera_speed * if sprint { 2.0 } else { 1.0 };
        }
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::D) {
            let right = camera.front.cross(&camera.up).normalize();
            camera.position = camera.position + right * camera_speed * if sprint { 2.0 } else { 1.0 };
        }
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Q) {
            camera.position = camera.position - camera.up * camera_speed * if sprint { 2.0 } else { 1.0 };
        }
        if keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::E) {
            camera.position = camera.position + camera.up * camera_speed * if sprint { 2.0 } else { 1.0 };
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main_loop,
                Event::MouseMotion { xrel, yrel, .. } => {
                    let xoffset = xrel as f32 * mouse_sensitivity;
                    let yoffset = -yrel as f32 * mouse_sensitivity;  // Reversed since y-coordinates go from bottom to top

                    camera.yaw += xoffset;
                    camera.pitch += yoffset;

                    // Constrain pitch
                    if camera.pitch > 89.0 {
                        camera.pitch = 89.0;
                    }
                    if camera.pitch < -89.0 {
                        camera.pitch = -89.0;
                    }

                    camera.update_camera_vectors();
                }
                Event::Window { win_event: sdl2::event::WindowEvent::FocusLost, .. } => {
                    // Release mouse when window loses focus
                    mouse.set_relative_mouse_mode(false);
                }
                Event::Window { win_event: sdl2::event::WindowEvent::FocusGained, .. } => {
                    // Capture mouse when window gains focus
                    mouse.set_relative_mouse_mode(true);
                }
                _ => {}
            }
        }

        // Render frame
        let view = camera.get_view_matrix();
        let model = Mat4::scale(Vec3::new(1.0, 1.0, 1.0));  // Changed scale to 1.0
        let transform = projection * view * model;

        gl_utils::clear_color(0.2, 0.3, 0.3, 1.0);
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            // Bind textures
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, grass_top_texture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, grass_side_texture);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, dirt_texture);
            gl::ActiveTexture(gl::TEXTURE3);
            gl::BindTexture(gl::TEXTURE_2D, colormap_texture);
            gl::ActiveTexture(gl::TEXTURE4);
            gl::BindTexture(gl::TEXTURE_2D, grass_side_overlay_texture);
            gl::ActiveTexture(gl::TEXTURE5);
            gl::BindTexture(gl::TEXTURE_2D, stone_texture);
            gl::ActiveTexture(gl::TEXTURE6);
            gl::BindTexture(gl::TEXTURE_2D, water_texture);

            let transform_loc = gl::GetUniformLocation(shader_program.0, b"transform\0".as_ptr() as *const i8);
            gl::UniformMatrix4fv(transform_loc, 1, gl::FALSE, transform.as_ptr());
        }

        shader_program.use_program();
        vao.bind();
        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                (all_indices.len() * 3) as i32,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
        }

        window.gl_swap_window();

        // Frame limiting
        let frame_time = timer.ticks() as f32 - current_frame_time;
        if frame_time < target_frame_time {
            thread::sleep(Duration::from_millis(((target_frame_time - frame_time) as u64).max(0)));
        }
    }
}
