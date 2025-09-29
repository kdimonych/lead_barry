//! Matrix operations module for embedded systems
//!
//! This module provides efficient matrix operations suitable for embedded environments
//! with limited memory and no heap allocation.

use defmt::*;
use micromath::F32Ext;
use nalgebra::{Matrix3, Matrix4, Vector2, Vector3, Vector4};

/// 2D transformation matrix for graphics operations
pub type Transform2D = Matrix3<f32>;

/// 3D transformation matrix for 3D graphics
pub type Transform3D = Matrix4<f32>;

/// Point in 2D space
pub type Point2D = Vector2<f32>;

/// Point in 3D space
pub type Point3D = Vector3<f32>;

/// Matrix operations for embedded systems
pub struct MatrixOps;

impl MatrixOps {
    /// Create a 2D rotation matrix
    pub fn rotation_2d(angle_rad: f32) -> Transform2D {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        Transform2D::new(cos_a, -sin_a, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 1.0)
    }

    /// Create a 2D translation matrix
    pub fn translation_2d(dx: f32, dy: f32) -> Transform2D {
        Transform2D::new(1.0, 0.0, dx, 0.0, 1.0, dy, 0.0, 0.0, 1.0)
    }

    /// Create a 2D scaling matrix
    pub fn scale_2d(sx: f32, sy: f32) -> Transform2D {
        Transform2D::new(sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0)
    }

    /// Transform a 2D point using a transformation matrix
    pub fn transform_point_2d(matrix: &Transform2D, point: Point2D) -> Point2D {
        let homogeneous = Vector3::new(point.x, point.y, 1.0);
        let transformed = matrix * homogeneous;
        Vector2::new(transformed.x, transformed.y)
    }

    /// Create a 3D rotation matrix around X axis
    pub fn rotation_x_3d(angle_rad: f32) -> Transform3D {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        Transform3D::new(
            1.0, 0.0, 0.0, 0.0, 0.0, cos_a, -sin_a, 0.0, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Create a 3D rotation matrix around Y axis
    pub fn rotation_y_3d(angle_rad: f32) -> Transform3D {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        Transform3D::new(
            cos_a, 0.0, sin_a, 0.0, 0.0, 1.0, 0.0, 0.0, -sin_a, 0.0, cos_a, 0.0, 0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Create a 3D rotation matrix around Z axis
    pub fn rotation_z_3d(angle_rad: f32) -> Transform3D {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        Transform3D::new(
            cos_a, -sin_a, 0.0, 0.0, sin_a, cos_a, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Create a 3D translation matrix
    pub fn translation_3d(dx: f32, dy: f32, dz: f32) -> Transform3D {
        Transform3D::new(
            1.0, 0.0, 0.0, dx, 0.0, 1.0, 0.0, dy, 0.0, 0.0, 1.0, dz, 0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Transform a 3D point using a transformation matrix
    pub fn transform_point_3d(matrix: &Transform3D, point: Point3D) -> Point3D {
        let homogeneous = Vector4::new(point.x, point.y, point.z, 1.0);
        let transformed = matrix * homogeneous;
        Vector3::new(transformed.x, transformed.y, transformed.z)
    }

    /// Simple linear interpolation between two points
    pub fn lerp_2d(a: Point2D, b: Point2D, t: f32) -> Point2D {
        a + t * (b - a)
    }

    /// Calculate distance between two 2D points
    pub fn distance_2d(a: Point2D, b: Point2D) -> f32 {
        let diff = b - a;
        (diff.x * diff.x + diff.y * diff.y).sqrt()
    }

    /// Calculate dot product of two 2D vectors
    pub fn dot_2d(a: Point2D, b: Point2D) -> f32 {
        a.dot(&b)
    }

    /// Normalize a 2D vector
    pub fn normalize_2d(v: Point2D) -> Point2D {
        let magnitude = (v.x * v.x + v.y * v.y).sqrt();
        if magnitude > 0.0 {
            Vector2::new(v.x / magnitude, v.y / magnitude)
        } else {
            Vector2::new(0.0, 0.0)
        }
    }
}

/// Kalman filter for sensor fusion (useful for IMU data)
pub struct KalmanFilter {
    /// State estimate
    pub x: f32,
    /// Estimate uncertainty
    pub p: f32,
    /// Process noise
    pub q: f32,
    /// Measurement noise
    pub r: f32,
}

impl KalmanFilter {
    /// Create a new Kalman filter
    pub fn new(
        initial_value: f32,
        initial_uncertainty: f32,
        process_noise: f32,
        measurement_noise: f32,
    ) -> Self {
        Self {
            x: initial_value,
            p: initial_uncertainty,
            q: process_noise,
            r: measurement_noise,
        }
    }

    /// Predict step
    pub fn predict(&mut self) {
        // Simple model: x_k = x_{k-1} (no control input)
        // P_k = P_{k-1} + Q
        self.p += self.q;
    }

    /// Update step with measurement
    pub fn update(&mut self, measurement: f32) {
        // Kalman gain: K = P / (P + R)
        let k = self.p / (self.p + self.r);

        // Update estimate: x = x + K * (z - x)
        self.x += k * (measurement - self.x);

        // Update uncertainty: P = (1 - K) * P
        self.p *= 1.0 - k;
    }

    /// Get current estimate
    pub fn estimate(&self) -> f32 {
        self.x
    }
}

/// Example matrix computations for display
pub fn demo_matrix_operations() {
    info!("=== Matrix Operations Demo ===");

    // 2D transformations
    let rotation = MatrixOps::rotation_2d(core::f32::consts::PI / 4.0); // 45 degrees
    let translation = MatrixOps::translation_2d(10.0, 20.0);
    let scale = MatrixOps::scale_2d(2.0, 2.0);

    // Combine transformations
    let combined = translation * rotation * scale;

    // Transform a point
    let point = Point2D::new(1.0, 0.0);
    let transformed = MatrixOps::transform_point_2d(&combined, point);

    info!("Original point: ({}, {})", point.x, point.y);
    info!("Transformed point: ({}, {})", transformed.x, transformed.y);

    // Vector operations
    let vec_a = Point2D::new(3.0, 4.0);
    let vec_b = Point2D::new(1.0, 2.0);

    let distance = MatrixOps::distance_2d(vec_a, vec_b);
    let dot_product = MatrixOps::dot_2d(vec_a, vec_b);
    let normalized = MatrixOps::normalize_2d(vec_a);

    info!("Distance: {}", distance);
    info!("Dot product: {}", dot_product);
    info!("Normalized vector: ({}, {})", normalized.x, normalized.y);

    // Kalman filter example
    let mut filter = KalmanFilter::new(0.0, 1.0, 0.1, 0.5);

    // Simulate some noisy measurements
    let measurements = [1.0, 1.2, 0.8, 1.1, 0.9];
    for measurement in measurements.iter() {
        filter.predict();
        filter.update(*measurement);
        info!(
            "Measurement: {}, Filtered: {}",
            measurement,
            filter.estimate()
        );
    }
}

/// Fixed-point matrix operations for even more efficiency
pub mod fixed_point {
    use heapless::Vec;

    /// Fixed-point number with 16.16 format
    #[derive(Clone, Copy, Debug)]
    pub struct Fixed16(i32);

    impl Fixed16 {
        const SCALE: i32 = 1 << 16;

        pub fn from_f32(val: f32) -> Self {
            Self((val * Self::SCALE as f32) as i32)
        }

        pub fn to_f32(self) -> f32 {
            self.0 as f32 / Self::SCALE as f32
        }

        pub fn mul(self, other: Self) -> Self {
            Self(((self.0 as i64 * other.0 as i64) >> 16) as i32)
        }

        pub fn add(self, other: Self) -> Self {
            Self(self.0 + other.0)
        }
    }

    /// 2x2 fixed-point matrix
    pub struct Matrix2x2Fixed {
        pub data: [Fixed16; 4],
    }

    impl Matrix2x2Fixed {
        pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
            Self {
                data: [
                    Fixed16::from_f32(a),
                    Fixed16::from_f32(b),
                    Fixed16::from_f32(c),
                    Fixed16::from_f32(d),
                ],
            }
        }

        pub fn multiply_vector(&self, x: f32, y: f32) -> (f32, f32) {
            let fx = Fixed16::from_f32(x);
            let fy = Fixed16::from_f32(y);

            let result_x = self.data[0].mul(fx).add(self.data[1].mul(fy));
            let result_y = self.data[2].mul(fx).add(self.data[3].mul(fy));

            (result_x.to_f32(), result_y.to_f32())
        }
    }
}
