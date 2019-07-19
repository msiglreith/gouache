use std::collections::HashMap;
use std::ffi::{CStr, CString};
use gl::types::{GLuint, GLint, GLchar, GLenum, GLvoid};

macro_rules! offset {
    ($type:ty, $field:ident) => { &(*(0 as *const $type)).$field as *const _ as usize }
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub path: [u16; 2],
}

pub enum TextureFormat { RGBA, A, RGB }
pub type TextureId = usize;

pub struct RenderOptions {
    pub target: Option<TextureId>,
}

impl Default for RenderOptions {
    fn default() -> RenderOptions {
        RenderOptions { target: None }
    }
}

pub struct Renderer {
    width: u32,
    height: u32,

    prog: Program,

    curves: Texture,

    textures: HashMap<TextureId, Texture>,
    texture_id: TextureId,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Renderer {
        let prog = Program::new(
            &CString::new(include_bytes!("shader/vert.glsl") as &[u8]).unwrap(),
            &CString::new(include_bytes!("shader/root.glsl") as &[u8]).unwrap()).unwrap();

        fn to_u16(x: f32) -> u16 { (((x + 100.0) / 1000.0) * std::u16::MAX as f32) as u16 }

        let font = ttf_parser::Font::from_data(include_bytes!("../res/sawarabi-gothic-medium.ttf"), 0).unwrap();
        struct Builder { first: Option<[f32; 2]>, last: [f32; 2], curves: Vec<[u16; 3]> }
        impl ttf_parser::glyf::OutlineBuilder for Builder {
            fn move_to(&mut self, x: f32, y: f32) {
                if self.first.is_none() {
                    self.first = Some([x, y]);
                }
                self.last = [x, y];
            }
            fn line_to(&mut self, x: f32, y: f32) {
                self.curves.push([to_u16(self.last[0]), to_u16(self.last[1]), to_u16(x)]);
                self.curves.push([to_u16(y), to_u16(x), to_u16(y)]);
                self.last = [x, y];
            }
            fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
                self.curves.push([to_u16(self.last[0]), to_u16(self.last[1]), to_u16(x1)]);
                self.curves.push([to_u16(y1), to_u16(x), to_u16(y)]);
                self.last = [x, y];
            }
            fn close(&mut self) {
                if let Some(first) = self.first {
                    self.line_to(first[0], first[1]);
                }
                self.first = None;
            }
        }
        let a = font.glyph_index('A').unwrap();
        let glyph = font.glyph(a).unwrap();
        let mut builder = Builder { first: None, last: [0.0, 0.0], curves: Vec::new() };
        glyph.outline(&mut builder);

        let x = font.glyph_index('8').unwrap();
        let glyph = font.glyph(x).unwrap();
        glyph.outline(&mut builder);
        println!("{}", builder.curves.len());


        let curves: [[u16; 3]; 8] = [
            [std::u16::MAX / 2, std::u16::MAX / 4, std::u16::MAX / 2],
            [std::u16::MAX / 2, std::u16::MAX, std::u16::MAX],
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
        ];

        let curves = Texture::new(TextureFormat::RGB, builder.curves.len() as u32, 1, &builder.curves);

        unsafe {
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::BLEND);
            gl::Enable(gl::FRAMEBUFFER_SRGB);
        }

        Renderer {
            width,
            height,

            prog,

            curves,

            textures: HashMap::new(),
            texture_id: 0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn clear(&mut self, col: [f32; 4], options: &RenderOptions) {
        self.apply_options(options);
        unsafe {
            gl::ClearColor(col[0], col[1], col[2], col[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.unapply_options(options);
    }

    pub fn draw(&mut self, vertices: &[Vertex], indices: &[u16], options: &RenderOptions) {
        let mut query: u32 = 0;
        unsafe {
            gl::GenQueries(1, &mut query);
            gl::BeginQuery(gl::TIME_ELAPSED, query);
        }

        self.apply_options(options);
        let vertex_array = VertexArray::new(vertices, indices);
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.curves.id);
            gl::UseProgram(self.prog.id);
            gl::Uniform1i(0, 0);
            gl::DrawElements(gl::TRIANGLES, vertex_array.count, gl::UNSIGNED_SHORT, 0 as *const GLvoid);
        }
        self.unapply_options(options);

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

    pub fn create_texture<T: Copy>(&mut self, format: TextureFormat, width: u32, height: u32, pixels: &[T]) -> TextureId {
        let id = self.texture_id;
        self.textures.insert(id, Texture::new(format, width, height, pixels));
        self.texture_id += 1;
        id
    }

    pub fn update_texture<T: Copy>(&mut self, texture: TextureId, x: u32, y: u32, width: u32, height: u32, pixels: &[T]) {
        self.textures.get_mut(&texture).unwrap().update(x, y, width, height, pixels);
    }

    pub fn delete_texture(&mut self, texture: TextureId) {
        self.textures.remove(&texture);
    }

    fn apply_options(&mut self, options: &RenderOptions) {
        if let Some(target) = options.target {
            let texture = self.textures.get_mut(&target).unwrap();
            if texture.framebuffer.is_none() {
                texture.framebuffer = Some(Framebuffer::new(texture.id));
            }
            unsafe {
                gl::Viewport(0, 0, texture.width as GLint, texture.height as GLint);
                gl::BindFramebuffer(gl::FRAMEBUFFER, texture.framebuffer.as_ref().unwrap().id);
            }
        }
    }

    fn unapply_options(&mut self, options: &RenderOptions) {
        if let Some(target) = options.target {
            unsafe {
                gl::Viewport(0, 0, self.width as GLint, self.height as GLint);
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
        }
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
        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, uv) as *const GLvoid);
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribIPointer(2, 2, gl::UNSIGNED_SHORT, std::mem::size_of::<Vertex>() as GLint, offset!(Vertex, path) as *const GLvoid);
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

struct Texture {
    id: GLuint,
    format: TextureFormat,
    width: u32,
    height: u32,
    framebuffer: Option<Framebuffer>,
}

impl Texture {
    fn new<T: Copy>(format: TextureFormat, width: u32, height: u32, pixels: &[T]) -> Texture {
        assert!(pixels.len() as u32 == width * height);

        let flipped = flip(pixels, width);
        let mut id: GLuint = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            match format {
                TextureFormat::RGBA => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::SRGB8_ALPHA8 as GLint, width as i32, height as i32, 0, gl::RGBA, gl::UNSIGNED_INT_8_8_8_8, pixels.as_ptr() as *const std::ffi::c_void);
                }
                TextureFormat::A => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::R16F as GLint, width as i32, height as i32, 0, gl::RED, gl::UNSIGNED_BYTE, pixels.as_ptr() as *const std::ffi::c_void);
                }
                TextureFormat::RGB => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 2);
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB16 as GLint, width as i32, height as i32, 0, gl::RGB, gl::UNSIGNED_SHORT, pixels.as_ptr() as *const std::ffi::c_void);
                }
            }
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        }
        Texture { id, format, width, height, framebuffer: None }
    }

    fn update<T: Copy>(&mut self, x: u32, y: u32, width: u32, height: u32, pixels: &[T]) {
        assert!(pixels.len() as u32 == width * height);

        let flipped = flip(pixels, width);
        unsafe { gl::BindTexture(gl::TEXTURE_2D, self.id); }
        match self.format {
            TextureFormat::RGBA => {
                unsafe {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
                    gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as i32, y as i32, width as i32, height as i32, gl::RGBA, gl::UNSIGNED_INT_8_8_8_8, pixels.as_ptr() as *const std::ffi::c_void);
                }
            }
            TextureFormat::A => {
                unsafe {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                    gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as i32, y as i32, width as i32, height as i32, gl::RED, gl::UNSIGNED_BYTE, pixels.as_ptr() as *const std::ffi::c_void);
                }
            }
            TextureFormat::RGB => {
                unsafe {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 2);
                    gl::TexSubImage2D(gl::TEXTURE_2D, 0, x as i32, y as i32, width as i32, height as i32, gl::RGB, gl::UNSIGNED_SHORT, pixels.as_ptr() as *const std::ffi::c_void);
                }
            }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}

fn flip<T: Copy>(pixels: &[T], width: u32) -> Vec<T> {
    let mut flipped: Vec<T> = Vec::with_capacity(pixels.len());
    for chunk in pixels.rchunks(width as usize) {
        flipped.extend_from_slice(chunk);
    }
    flipped
}

struct Framebuffer {
    id: GLuint,
}

impl Framebuffer {
    fn new(texture_id: GLuint) -> Framebuffer {
        let mut id: GLuint = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, id);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture_id, 0);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }
        Framebuffer { id }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe { gl::DeleteFramebuffers(1, &self.id); }
    }
}
