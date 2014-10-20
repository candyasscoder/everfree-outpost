use v3::V3;
use super::CollideResult;

pub struct CollideArgs {
    pos: V3,
    size: V3,
    velocity: V3,
}

#[export_name = "collide"]
pub extern fn collide_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = super::collide(input.pos, input.size, input.velocity);
}

#[export_name = "collide_ramp"]
pub extern fn collide_ramp_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = super::collide_ramp(input.pos, input.size, input.velocity);
}

pub struct IsOnRampArgs {
    pos: V3,
    size: V3,
}

#[export_name = "get_ramp_angle"]
pub extern fn get_ramp_angle_wrapper(input: &IsOnRampArgs, output: &mut i32) {
    *output = super::get_ramp_angle(input.pos, input.size) as i32;
}

#[export_name = "get_next_ramp_angle"]
pub extern fn get_next_ramp_angle_wrapper(input: &CollideArgs, output: &mut i32) {
    *output = super::get_next_ramp_angle(input.pos, input.size, input.velocity) as i32;
}
