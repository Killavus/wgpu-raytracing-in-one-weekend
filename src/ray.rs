use crate::types::*;
use encase::ShaderType;

#[derive(ShaderType)]
pub struct Ray {
    origin: Vec3,
    direction: Vec3,
    finished: u32,
}
