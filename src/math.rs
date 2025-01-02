use std::ops::{Add, Mul, Sub};

/// A 3D vector type
#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Vec3 {
        let len = self.length();
        if len != 0.0 {
            Vec3 {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, scalar: f32) -> Vec3 {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

/// A 4x4 matrix stored in column-major order
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    data: [f32; 16],
}

impl Mat4 {
    pub fn new(data: [f32; 16]) -> Self {
        Self { data }
    }

    pub fn identity() -> Self {
        let mut data = [0.0; 16];
        data[0] = 1.0;
        data[5] = 1.0;
        data[10] = 1.0;
        data[15] = 1.0;
        Self { data }
    }

    pub fn translate(translation: Vec3) -> Self {
        let mut result = Self::identity();
        result.data[12] = translation.x;
        result.data[13] = translation.y;
        result.data[14] = translation.z;
        result
    }

    pub fn scale(scale: Vec3) -> Self {
        let mut result = Self::identity();
        result.data[0] = scale.x;
        result.data[5] = scale.y;
        result.data[10] = scale.z;
        result
    }

    pub fn rotate(angle_radians: f32, axis: Vec3) -> Self {
        let axis = axis.normalize();
        let sin = angle_radians.sin();
        let cos = angle_radians.cos();
        let one_minus_cos = 1.0 - cos;

        let mut result = Self::identity();
        
        result.data[0] = cos + axis.x * axis.x * one_minus_cos;
        result.data[1] = axis.x * axis.y * one_minus_cos + axis.z * sin;
        result.data[2] = axis.x * axis.z * one_minus_cos - axis.y * sin;

        result.data[4] = axis.y * axis.x * one_minus_cos - axis.z * sin;
        result.data[5] = cos + axis.y * axis.y * one_minus_cos;
        result.data[6] = axis.y * axis.z * one_minus_cos + axis.x * sin;

        result.data[8] = axis.z * axis.x * one_minus_cos + axis.y * sin;
        result.data[9] = axis.z * axis.y * one_minus_cos - axis.x * sin;
        result.data[10] = cos + axis.z * axis.z * one_minus_cos;

        result
    }

    pub fn perspective(fov_y_radians: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_radians / 2.0).tan();
        let mut result = Self::identity();
        
        result.data[0] = f / aspect;
        result.data[5] = f;
        result.data[10] = (far + near) / (near - far);
        result.data[11] = -1.0;
        result.data[14] = (2.0 * far * near) / (near - far);
        result.data[15] = 0.0;
        
        result
    }

    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> Self {
        let z = (position - target).normalize();
        let x = up.cross(&z).normalize();
        let y = z.cross(&x);

        let mut result = Mat4::identity();
        
        // First three columns are the right, up, and forward vectors
        result.data[0] = x.x;
        result.data[1] = y.x;
        result.data[2] = z.x;
        
        result.data[4] = x.y;
        result.data[5] = y.y;
        result.data[6] = z.y;
        
        result.data[8] = x.z;
        result.data[9] = y.z;
        result.data[10] = z.z;
        
        // Last column is the negated and transformed position
        result.data[12] = -x.dot(&position);
        result.data[13] = -y.dot(&position);
        result.data[14] = -z.dot(&position);

        result
    }

    pub fn as_ptr(&self) -> *const f32 {
        self.data.as_ptr()
    }
}

impl Mul for Mat4 {
    type Output = Mat4;

    fn mul(self, other: Mat4) -> Mat4 {
        let mut result = Mat4::identity();
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.data[i + k * 4] * other.data[k + j * 4];
                }
                result.data[i + j * 4] = sum;
            }
        }
        result
    }
}

impl Mul<Vec3> for Mat4 {
    type Output = Vec3;

    fn mul(self, vec: Vec3) -> Vec3 {
        let x = self.data[0] * vec.x + self.data[4] * vec.y + self.data[8] * vec.z + self.data[12];
        let y = self.data[1] * vec.x + self.data[5] * vec.y + self.data[9] * vec.z + self.data[13];
        let z = self.data[2] * vec.x + self.data[6] * vec.y + self.data[10] * vec.z + self.data[14];
        let w = self.data[3] * vec.x + self.data[7] * vec.y + self.data[11] * vec.z + self.data[15];

        if w != 1.0 && w != 0.0 {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }
} 

