use std::time::Instant;
use std::sync::Mutex;
use std::f32;
use nalgebra::{Quaternion, Unit, UnitQuaternion, Vector3};
use macroquad::prelude::*;
use lazy_static::lazy_static;

// 陀螺仪数据结构
#[derive(Debug, Clone, Copy)]
pub struct GyroData {
    pub angular_velocity: Vector3<f32>, // 角速度 (rad/s)
    pub timestamp: Instant,             // 数据时间戳
}

// 画面稳定器
pub struct Gyro {
    rotation: UnitQuaternion<f32>,
    last_gyro_data: Option<GyroData>,
}

lazy_static! {
    pub static ref GYRO_SCOPE_DATA: Mutex<GyroData> = Mutex::new(GyroData {
        angular_velocity: Vector3::new(0.0, 0.0, 0.0),
        timestamp: Instant::now()
    });
    pub static ref ROTATION_VECTOR_DATA: Mutex<Vector3<f32>> = Mutex::new(Vector3::new(0.0, 0.0, 0.0));
    pub static ref GYRO: Mutex<Gyro> = Mutex::new(Gyro::new());
}

impl Gyro {
    pub fn new() -> Self {
        let rotation_vector = ROTATION_VECTOR_DATA.lock().unwrap();
        let origin = UnitQuaternion::from_euler_angles(0.0, f32::consts::PI / 2.0, 0.0) * UnitQuaternion::from_euler_angles(rotation_vector.x, rotation_vector.y, 0.0);
        Self {
            rotation: origin,
            last_gyro_data: None,
        }
    }

    pub fn update(&mut self, gyro_data: GyroData) {
        if let Some(last) = self.last_gyro_data {
            let dt = gyro_data.timestamp
                .duration_since(last.timestamp)
                .as_secs_f32();

            let omega = gyro_data.angular_velocity;
            let angle = omega.norm() * dt;

            if angle > 0.0 {
                let axis_unit: Unit<Vector3<f32>> = Unit::new_normalize(omega);
                let dq = UnitQuaternion::from_axis_angle(&axis_unit, angle); // 增量四元数
                self.rotation = self.rotation * dq;
            }
        }
        self.last_gyro_data = Some(gyro_data);
}

    pub fn get_angle(&self) -> f32 {
        let rotation_matrix = self.rotation.to_rotation_matrix().euler_angles();
        debug!("RotationMatrix: {:+.7}, {:+.7}, {:+.7}", rotation_matrix.0, rotation_matrix.1, rotation_matrix.2);
        rotation_matrix.2
    }

    pub fn reset(&mut self) {
        let rotation_vector = ROTATION_VECTOR_DATA.lock().unwrap();
        let origin = UnitQuaternion::from_euler_angles(0.0, f32::consts::PI / 2.0, 0.0) * UnitQuaternion::from_euler_angles(rotation_vector.x, rotation_vector.y, 0.0);
        self.rotation = origin;
        let q1 = origin.to_rotation_matrix().euler_angles();
        debug!("RotationVector: {:+.7}, {:+.7}, {:+.7}", q1.0, q1.1, q1.2);
    }
}
