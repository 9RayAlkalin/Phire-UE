use std::time::Instant;
use std::sync::Mutex;
use nalgebra::{Vector3, UnitQuaternion, Quaternion};
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
        let rotation_vector = ROTATION_VECTOR_DATA.lock().unwrap().clone();
        let delta_quat = UnitQuaternion::from_euler_angles(
            rotation_vector.x, 
            rotation_vector.y, 
            rotation_vector.z
        );
        Self {
            rotation: delta_quat,
            last_gyro_data: None,
        }
    }

    // 更新陀螺仪数据并计算旋转
    pub fn update(&mut self, gyro_data: GyroData) {
        if let Some(last_data) = self.last_gyro_data {
            let dt = gyro_data.timestamp.duration_since(last_data.timestamp).as_secs_f32();
            
            let angle_delta = gyro_data.angular_velocity * dt;
            
            // 四元数增量
            let delta_quat = UnitQuaternion::from_euler_angles(
                angle_delta.x, 
                angle_delta.y, 
                angle_delta.z
            );
            
            self.rotation = self.rotation * delta_quat;
        }
        
        self.last_gyro_data = Some(gyro_data);
    }

    // 应用旋转补偿到视图矩阵
    pub fn apply(&self) -> f32 {
        let rotation_matrix = self.rotation.to_rotation_matrix().euler_angles();
        debug!("rotation_matrix: {:?}", rotation_matrix);
        rotation_matrix.2
    }

    pub fn reset(&mut self) {
        let rotation_vector = ROTATION_VECTOR_DATA.lock().unwrap().clone();
        let delta_quat = UnitQuaternion::from_euler_angles(
            rotation_vector.x, 
            rotation_vector.y, 
            rotation_vector.z
        );
        self.rotation = UnitQuaternion::identity();
    }
}
