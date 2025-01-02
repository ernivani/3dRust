use gl::types::{GLenum, GLint, GLsizei, GLuint};
use std::ffi::CString;
use image::GenericImageView;

/// Sets the color to clear to when clearing the screen.
pub fn clear_color(r: f32, g: f32, b: f32, a: f32) {
    unsafe { gl::ClearColor(r, g, b, a) }
}

/// The types of shader object.
#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    /// Vertex shaders determine the position of geometry within the screen.
    Vertex = gl::VERTEX_SHADER as isize,
    /// Fragment shaders determine the color output of geometry.
    Fragment = gl::FRAGMENT_SHADER as isize,
}

/// The types of buffer object that you can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// Array Buffers holds arrays of vertex data for drawing.
    Array = gl::ARRAY_BUFFER as isize,
    /// Element Array Buffers hold indexes of what vertexes to use for drawing.
    ElementArray = gl::ELEMENT_ARRAY_BUFFER as isize,
}

/// Basic wrapper for a Vertex Array Object.
pub struct VertexArray(pub GLuint);
impl VertexArray {
    /// Creates a new vertex array object
    pub fn new() -> Option<Self> {
        let mut vao = 0;
        unsafe { gl::GenVertexArrays(1, &mut vao) };
        if vao != 0 {
            Some(Self(vao))
        } else {
            None
        }
    }

    /// Bind this vertex array as the current vertex array object
    pub fn bind(&self) {
        unsafe { gl::BindVertexArray(self.0) }
    }

    /// Clear the current vertex array object binding.
    pub fn clear_binding() {
        unsafe { gl::BindVertexArray(0) }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe { gl::DeleteVertexArrays(1, &self.0) }
    }
}

/// Basic wrapper for a Buffer Object.
pub struct Buffer(pub GLuint);
impl Buffer {
    /// Makes a new buffer
    pub fn new() -> Option<Self> {
        let mut vbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }
        if vbo != 0 {
            Some(Self(vbo))
        } else {
            None
        }
    }

    /// Bind this buffer for the given type
    pub fn bind(&self, ty: BufferType) {
        unsafe { gl::BindBuffer(ty as GLenum, self.0) }
    }

    /// Clear the current buffer binding for the given type.
    pub fn clear_binding(ty: BufferType) {
        unsafe { gl::BindBuffer(ty as GLenum, 0) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { gl::DeleteBuffers(1, &self.0) }
    }
}

/// Places a slice of data into a previously-bound buffer.
pub fn buffer_data(ty: BufferType, data: &[u8], usage: GLenum) {
    unsafe {
        gl::BufferData(
            ty as GLenum,
            data.len().try_into().unwrap(),
            data.as_ptr().cast(),
            usage,
        );
    }
}

/// A handle to a Shader Object
pub struct Shader(pub GLuint);
impl Shader {
    /// Makes a new shader.
    pub fn new(ty: ShaderType) -> Option<Self> {
        let shader = unsafe { gl::CreateShader(ty as GLenum) };
        if shader != 0 {
            Some(Self(shader))
        } else {
            None
        }
    }

    /// Assigns a source string to the shader.
    pub fn set_source(&self, src: &str) {
        unsafe {
            let c_str = CString::new(src.as_bytes()).unwrap();
            gl::ShaderSource(self.0, 1, &c_str.as_ptr(), std::ptr::null());
        }
    }

    /// Compiles the shader based on the current source.
    pub fn compile(&self) {
        unsafe { gl::CompileShader(self.0) };
    }

    /// Checks if the last compile was successful or not.
    pub fn compile_success(&self) -> bool {
        let mut compiled = 0;
        unsafe { gl::GetShaderiv(self.0, gl::COMPILE_STATUS, &mut compiled) };
        compiled == gl::TRUE as GLint
    }

    /// Gets the info log for the shader.
    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { gl::GetShaderiv(self.0, gl::INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            gl::GetShaderInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Takes a shader type and source string and produces either the compiled
    /// shader or an error message.
    pub fn from_source(ty: ShaderType, source: &str) -> Result<Self, String> {
        let id = Self::new(ty).ok_or_else(|| "Couldn't allocate new shader".to_string())?;
        id.set_source(source);
        id.compile();
        if id.compile_success() {
            Ok(id)
        } else {
            let out = id.info_log();
            Err(out)
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { gl::DeleteShader(self.0) }
    }
}

/// A handle to a Program Object
pub struct ShaderProgram(pub GLuint);
impl ShaderProgram {
    /// Allocates a new program object.
    pub fn new() -> Option<Self> {
        let prog = unsafe { gl::CreateProgram() };
        if prog != 0 {
            Some(Self(prog))
        } else {
            None
        }
    }

    /// Attaches a shader object to this program object.
    pub fn attach_shader(&self, shader: &Shader) {
        unsafe { gl::AttachShader(self.0, shader.0) };
    }

    /// Links the various attached, compiled shader objects into a usable program.
    pub fn link_program(&self) {
        unsafe { gl::LinkProgram(self.0) };
    }

    /// Checks if the last linking operation was successful.
    pub fn link_success(&self) -> bool {
        let mut success = 0;
        unsafe { gl::GetProgramiv(self.0, gl::LINK_STATUS, &mut success) };
        success == gl::TRUE as GLint
    }

    /// Gets the log data for this program.
    pub fn info_log(&self) -> String {
        let mut needed_len = 0;
        unsafe { gl::GetProgramiv(self.0, gl::INFO_LOG_LENGTH, &mut needed_len) };
        let mut v: Vec<u8> = Vec::with_capacity(needed_len.try_into().unwrap());
        let mut len_written = 0_i32;
        unsafe {
            gl::GetProgramInfoLog(
                self.0,
                v.capacity().try_into().unwrap(),
                &mut len_written,
                v.as_mut_ptr().cast(),
            );
            v.set_len(len_written.try_into().unwrap());
        }
        String::from_utf8_lossy(&v).into_owned()
    }

    /// Sets the program as the program to use when drawing.
    pub fn use_program(&self) {
        unsafe { gl::UseProgram(self.0) };
    }

    /// Takes a vertex shader source string and a fragment shader source string
    /// and either gets you a working program object or gets you an error message.
    pub fn from_vert_frag(vert: &str, frag: &str) -> Result<Self, String> {
        let p = Self::new().ok_or_else(|| "Couldn't allocate a program".to_string())?;
        let v = Shader::from_source(ShaderType::Vertex, vert)
            .map_err(|e| format!("Vertex Compile Error: {}", e))?;
        let f = Shader::from_source(ShaderType::Fragment, frag)
            .map_err(|e| format!("Fragment Compile Error: {}", e))?;
        p.attach_shader(&v);
        p.attach_shader(&f);
        p.link_program();
        if p.link_success() {
            Ok(p)
        } else {
            let out = format!("Program Link Error: {}", p.info_log());
            Err(out)
        }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.0) }
    }
}

pub fn load_texture(path: &str) -> GLuint {
    let mut texture = 0;
    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        
        // Set texture wrapping/filtering options
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);

        // Load and generate the texture
        let img = image::open(path).expect("Failed to load texture");
        let data = img.to_rgba8();
        
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            img.width() as i32,
            img.height() as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const _
        );
    }
    texture
} 