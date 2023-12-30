use crate::types::*;
use encase::ShaderType;

#[derive(ShaderType)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    finished: u32,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Ray {
            origin,
            direction,
            finished: 0,
        }
    }
}
