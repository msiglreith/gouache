pub trait Renderer {
    fn clear(&mut self, color: [f32; 4]);
    fn draw(&mut self, vertices: &[Vertex], indices: &[u16]);
    fn upload_indices(&mut self, index: u16, indices: &[u16]);
    fn upload_vertices(&mut self, index: u16, vertices: &[u16]);
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub col: [f32; 4],
    pub uv: [f32; 2],
    pub path: [u16; 3],
}
