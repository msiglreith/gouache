use std::ffi::{CStr, CString};
use gl::types::{GLuint, GLint, GLchar, GLenum, GLvoid};

use crate::renderer::*;

macro_rules! offset {
    ($type:ty, $field:ident) => { &(*(0 as *const $type)).$field as *const _ as usize }
}

pub struct GlRenderer {
    prog: Program,
    paths: Texture<[u16; 4]>,
}

impl GlRenderer {
    pub fn new() -> GlRenderer {
        let prog = Program::new(
            &CString::new(include_bytes!("../shader/vert.glsl") as &[u8]).unwrap(),
            &CString::new(include_bytes!("../shader/frag.glsl") as &[u8]).unwrap()).unwrap();

        let paths = Texture::new(16384, 1, None);

        unsafe {
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
            gl::Enable(gl::FRAMEBUFFER_SRGB);
        }

        GlRenderer { prog, paths }
    }
}

impl Renderer for GlRenderer {
    fn clear(&mut self, col: [f32; 4]) {
        unsafe {
            gl::ClearColor(col[0], col[1], col[2], col[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    fn draw(&mut self, vertices: &[Vertex], indices: &[u16]) {
        let mut query: u32 = 0;
        unsafe {
            gl::GenQueries(1, &mut query);
            gl::BeginQuery(gl::TIME_ELAPSED, query);
        }

        let vertex_array = VertexArray::new(vertices, indices);
        unsafe {
            gl::UseProgram(self.prog.id);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.paths.id);
            gl::Uniform1i(0, 0);

            gl::DrawElements(gl::TRIANGLES, vertex_array.count, gl::UNSIGNED_SHORT, 0 as *const GLvoid);
        }

        let mut elapsed: u64 = 0;
        unsafe {
            gl::EndQuery(gl::TIME_ELAPSED);
            let mut available: i32 = 0;
            while available == 0 {
                gl::GetQueryObjectiv(query, gl::QUERY_RESULT_AVAILABLE, &mut available);
            }
            gl::GetQueryObjectui64v(query, gl::QUERY_RESULT, &mut elapsed);
        }

        println!("{}", elapsed);
    }

    fn upload(&mut self, index: u16, paths: &[[u16; 4]]) {
        assert!(index % 2 == 0);
        assert!(paths.len() % 2 == 0);
        self.paths.update(index as u32, 0, paths.len() as u32, 1, paths);
    }
}

struct Program {
    id: GLuint,
}

impl Program {
    fn new(vert_src: &CStr, frag_src: &CStr) -> Result<Program, String> {
        unsafe {
            let vert = shader(vert_src, gl::VERTEX_SHADER).unwrap();
            let frag = shader(frag_src, gl::FRAGMENT_SHADER).unwrap();
            let prog = gl::CreateProgram();
            gl::AttachShader(prog, vert);
            gl::AttachShader(prog, frag);
            gl::LinkProgram(prog);

            let mut valid: GLint = 1;
            gl::GetProgramiv(prog, gl::COMPILE_STATUS, &mut valid);
            if valid == 0 {
                let mut len: GLint = 0;
                gl::GetProgramiv(prog, gl::INFO_LOG_LENGTH, &mut len);
                let error = CString::new(vec![b' '; len as usize]).unwrap();
                gl::GetProgramInfoLog(prog, len, std::ptr::null_mut(), error.as_ptr() as *mut GLchar);
                return Err(error.into_string().unwrap());
            }

            gl::DetachShader(prog, vert);
            gl::DetachShader(prog, frag);

            gl::DeleteShader(vert);
            gl::DeleteShader(frag);

            Ok(Program { id: prog })
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}

fn shader(shader_src: &CStr, shader_type: GLenum) -> Result<GLuint, String> {
    unsafe {
        let shader: GLuint = gl::CreateShader(shader_type);
        gl::ShaderSource(shader, 1, &shader_src.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut valid: GLint = 1;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut valid);
        if valid == 0 {
            let mut len: GLint = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let error = CString::new(vec![b' '; len as usize]).unwrap();
            gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), error.as_ptr() as *mut GLchar);
            return Err(error.into_string().unwrap());
        }

        Ok(shader)
    }
}

trait VertexAttribs {
    unsafe fn attribs();
}

impl VertexAttribs for Vertex {
    unsafe fn attribs() {
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, pos) as *const GLvoid);
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, col) as *const GLvoid);
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, uv) as *const GLvoid);
        gl::EnableVertexAttribArray(3);
        gl::VertexAttribIPointer(3, 2, gl::UNSIGNED_SHORT, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, path) as *const GLvoid);
    }
}

struct VertexArray<V> {
    vao: GLuint,
    vbo: GLuint,
    ibo: GLuint,
    count: i32,
    phantom: std::marker::PhantomData<V>,
}

impl<V: VertexAttribs> VertexArray<V> {
    fn new(vertices: &[V], indices: &[u16]) -> VertexArray<V> {
        let mut vbo: u32 = 0;
        let mut ibo: u32 = 0;
        let mut vao: u32 = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER, (vertices.len() * std::mem::size_of::<V>()) as isize, vertices.as_ptr() as *const std::ffi::c_void, gl::DYNAMIC_DRAW);

            gl::GenBuffers(1, &mut ibo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, (indices.len() * std::mem::size_of::<u16>()) as isize, indices.as_ptr() as *const std::ffi::c_void, gl::DYNAMIC_DRAW);

            V::attribs();
        }
        VertexArray { vao, vbo, ibo, count: indices.len() as i32, phantom: std::marker::PhantomData }
    }
}

impl<V> Drop for VertexArray<V> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.ibo);
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}

trait Texel {
    const INTERNAL_FORMAT: GLint;
    const FORMAT: GLenum;
    const TYPE: GLenum;
}

impl Texel for [u16; 4] {
    const INTERNAL_FORMAT: GLint = gl::RGBA16 as GLint;
    const FORMAT: GLenum = gl::RGBA;
    const TYPE: GLenum = gl::UNSIGNED_SHORT;
}

struct Texture<P> {
    id: GLuint,
    phantom: std::marker::PhantomData<P>,
}

impl<P: Texel> Texture<P> {
    fn new(width: u32, height: u32, pixels: Option<&[P]>) -> Texture<P> where P: Copy {
        let data = if let Some(pixels) = pixels {
            assert!(pixels.len() as u32 == width * height);
            let flipped = flip(pixels, width);
            flipped.as_ptr() as *const std::ffi::c_void
        } else {
            std::ptr::null()
        };

        let mut id: GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, std::mem::align_of::<P>() as i32);
            gl::TexImage2D(gl::TEXTURE_2D, 0, P::INTERNAL_FORMAT, width as i32, height as i32, 0, P::FORMAT, P::TYPE, data);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }

        Texture { id, phantom: std::marker::PhantomData }
    }

    fn update<T: Copy>(&mut self, x: u32, y: u32, width: u32, height: u32, pixels: &[T]) {
        assert!(pixels.len() as u32 == width * height);
        let flipped = flip(pixels, width);
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, std::mem::align_of::<P>() as i32);
            gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as i32, y as i32, width as i32, height as i32, P::FORMAT, P::TYPE, flipped.as_ptr() as *const std::ffi::c_void);
        }
    }
}

impl<P> Drop for Texture<P> {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}

fn flip<P: Copy>(pixels: &[P], width: u32) -> Vec<P> {
    let mut flipped: Vec<P> = Vec::with_capacity(pixels.len());
    for chunk in pixels.rchunks(width as usize) {
        flipped.extend_from_slice(chunk);
    }
    flipped
}
