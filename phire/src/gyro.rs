use std::time::Instant;
use std::sync::Mutex;
use std::f32;
use nalgebra::{Unit, UnitQuaternion, Vector3};
use lazy_static::lazy_static;
use tracing::debug;

use crate::config::Config;

#[derive(Debug, Clone, Copy)]
pub struct GyroData {
    pub angular_velocity: Vector3<f32>, // 角速度 (rad/s)
    pub timestamp: Instant,             // 数据时间戳
}

pub struct Gyro {
    rotation_gravity: UnitQuaternion<f32>,

    rotation_gyroscope: UnitQuaternion<f32>,
    last_gyroscope_data: Option<GyroData>,
}

lazy_static! {
    pub static ref GYROSCOPE_DATA: Mutex<GyroData> = Mutex::new(GyroData {
        angular_velocity: Vector3::new(0.0, 0.0, 0.0),
        timestamp: Instant::now()
    });
    pub static ref GYRO: Mutex<Gyro> = Mutex::new(Gyro::new());
}

impl Gyro {
    pub fn new() -> Self {
        Self {
            rotation_gravity: UnitQuaternion::identity(),

            rotation_gyroscope: UnitQuaternion::identity(),
            last_gyroscope_data: None,
        }
    }

    pub(crate) fn reset_gyroscope(&mut self) {
        self.rotation_gyroscope = UnitQuaternion::identity();
    }

    pub fn update_gyroscope(&mut self, gyro_data: GyroData) {
        if let Some(last) = self.last_gyroscope_data {
            let dt = gyro_data.timestamp
                .duration_since(last.timestamp)
                .as_secs_f32();

            let omega = gyro_data.angular_velocity;
            let angle = omega.norm() * dt;

            if angle > 0.0 {
                let axis_unit: Unit<Vector3<f32>> = Unit::new_normalize(omega);
                let dq = UnitQuaternion::from_axis_angle(&axis_unit, angle); // 增量
                self.rotation_gyroscope *= dq;
            }
        }
        self.last_gyroscope_data = Some(gyro_data);
    }

    pub fn update_gravity(&mut self, gravity_data: Vector3<f32>) {
        let norm = gravity_data.norm();
        if norm == 0.0 {
            return;
        }

        let g_dev = gravity_data / norm;  // 归一化, g_dev 指向重力方向

        let world_gravity = Vector3::new(0.0, -1.0, 0.0); // 世界坐标系下的重力方向

        // g_dev 到 world_gravity 的旋转
        let q = UnitQuaternion::rotation_between(&g_dev, &world_gravity)
            .unwrap_or_else(UnitQuaternion::identity);

        self.rotation_gravity = q;
    }

    fn get_gyroscope_angle(&self) -> f32 {
        let (_, _, yaw) = self.rotation_gyroscope.to_rotation_matrix().euler_angles();
        yaw
    }

    fn get_gravity_angle(&self) -> f32 {
        let world = self.rotation_gravity.transform_vector(&Vector3::new(0.0, 1.0, 0.0));
        //let device = self.rotation.transform_vector(&Vector3::new(1.0, 0.0, 0.0));

        let proj: nalgebra::Matrix<f32, nalgebra::Const<3>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 3, 1>> = Vector3::new(world.x, world.y, world.z);
        let tan = world.y.atan2(proj.x);
        tan
    }

    pub fn get_angle(&self, config: &Config) -> f32 {
        if config.rotation_mode {
            if config.rotation_flat_mode {
                self.get_gyroscope_angle()
            } else {
                self.get_gravity_angle()
            }
        } else {
            0.0
        }
    }
}
