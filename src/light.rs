use crate::worldmachine::components::COMPONENT_TYPE_LIGHT;
use crate::worldmachine::ecs::{Component, ParameterValue};
use gfx_maths::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub casts_shadow: bool,
}

impl Light {
    pub fn from_component(component: Component) -> Option<Light> {
        if component.get_type() == COMPONENT_TYPE_LIGHT.clone() {
            let position = component.get_parameter("position");
            let position = match position.value {
                ParameterValue::Vec3(position) => position,
                _ => panic!("Invalid parameter type for position"),
            };
            let color = component.get_parameter("colour");
            let color = match color.value {
                ParameterValue::Vec3(color) => color,
                _ => panic!("Invalid parameter type for colour"),
            };
            let intensity = component.get_parameter("intensity");
            let intensity = match intensity.value {
                ParameterValue::Float(intensity) => intensity as f32,
                _ => panic!("Invalid parameter type for intensity"),
            };
            let radius = component.get_parameter("radius");
            let radius = match radius.value {
                ParameterValue::Float(radius) => radius as f32,
                _ => panic!("Invalid parameter type for intensity"),
            };
            let casts_shadow = component.get_parameter("casts_shadow");
            let casts_shadow = match casts_shadow.value {
                ParameterValue::Bool(casts_shadow) => casts_shadow,
                _ => panic!("Invalid parameter type for casts_shadow"),
            };
            Some(Light {
                position,
                color,
                intensity,
                radius,
                casts_shadow,
            })
        } else {
            None
        }
    }
}