pub trait Renderer {
    fn clear(&mut self, color: [f32; 4]);
    fn draw(&mut self, vertices: &[Vertex], indices: &[u16]);
    fn upload(&mut self, index: u16, paths: &[[u16; 3]]);
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub col: [f32; 4],
    pub uv: [f32; 2],
    pub path: [u16; 2],
}
