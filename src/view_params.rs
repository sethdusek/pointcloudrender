use nalgebra::{Matrix4, Point3, Vector3};

#[derive(Copy, Clone)]
pub struct ViewParams {
    eye: Point3<f32>,
    look_at: Point3<f32>,
    roll: f32,
    pitch: f32,
    yaw: f32,
    pub camera: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}

impl ViewParams {
    pub fn new(eye: Point3<f32>, look_at: Point3<f32>, projection: Matrix4<f32>) -> Self {
        ViewParams {
            eye,
            look_at,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            camera: Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0))
                * Matrix4::from_euler_angles(0.0, 0.0, 0.0),
            projection,
        }
    }

    fn update_camera(&mut self) {
        self.camera = Matrix4::look_at_rh(&self.eye, &self.look_at, &Vector3::new(0.0, 1.0, 0.0))
            * Matrix4::from_euler_angles(self.roll, self.pitch, self.yaw);
    }

    pub fn set_eye(&mut self, eye: Point3<f32>) {
        self.eye = eye;
        self.update_camera();
    }

    pub fn set_look_at(&mut self, look_at: Point3<f32>) {
        self.look_at = look_at;
        self.update_camera();
    }

    pub fn set_roll(&mut self, roll: f32) {
        self.roll = roll;
        self.update_camera();
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch;
        self.update_camera();
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.update_camera();
    }

    pub fn roll(&self) -> f32 {
        self.roll
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }
}
