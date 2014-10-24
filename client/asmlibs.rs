#![crate_name = "asmlibs"]
#![no_std]

extern crate physics;

use physics::v3::V3;
use physics::CollideResult;

pub struct CollideArgs {
    pub pos: V3,
    pub size: V3,
    pub velocity: V3,
}

#[export_name = "collide"]
pub extern fn collide_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = physics::physics2::collide(input.pos, input.size, input.velocity);
}

#[export_name = "collide_ramp"]
pub extern fn collide_ramp_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = physics::collide_ramp(input.pos, input.size, input.velocity);
}

pub struct IsOnRampArgs {
    pub pos: V3,
    pub size: V3,
}

#[export_name = "get_ramp_angle"]
pub extern fn get_ramp_angle_wrapper(input: &IsOnRampArgs, output: &mut i32) {
    *output = physics::get_ramp_angle(input.pos, input.size) as i32;
}

#[export_name = "get_next_ramp_angle"]
pub extern fn get_next_ramp_angle_wrapper(input: &CollideArgs, output: &mut i32) {
    *output = physics::get_next_ramp_angle(input.pos, input.size, input.velocity) as i32;
}
