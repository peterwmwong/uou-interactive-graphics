use bitflags::bitflags;
use metal::{FunctionConstantValues, MTLDataType};

use crate::UserEvent;

bitflags! {
    pub struct ShadingModeSelector: usize {
        const HAS_AMBIENT = 1 << 0;
        const HAS_DIFFUSE = 1 << 1;
        const ONLY_NORMALS = 1 << 2;
        const HAS_SPECULAR = 1 << 3;
        const DEFAULT = Self::HAS_AMBIENT.bits | Self::HAS_DIFFUSE.bits | Self::HAS_SPECULAR.bits;
    }
}

impl ShadingModeSelector {
    pub fn encode(
        &self,
        function_constants: FunctionConstantValues,
        ambient_function_constant_id: usize,
        diffuse_function_constant_id: usize,
        specular_function_constant_id: usize,
        normal_function_constant_id: usize,
    ) -> FunctionConstantValues {
        for (mode, id) in [
            (
                ShadingModeSelector::HAS_AMBIENT,
                ambient_function_constant_id,
            ),
            (
                ShadingModeSelector::HAS_DIFFUSE,
                diffuse_function_constant_id,
            ),
            (
                ShadingModeSelector::HAS_SPECULAR,
                specular_function_constant_id,
            ),
            (
                ShadingModeSelector::ONLY_NORMALS,
                normal_function_constant_id,
            ),
        ] {
            function_constants.set_constant_value_at_index(
                (&self.contains(mode) as *const bool) as _,
                MTLDataType::Bool,
                id as _,
            );
        }
        function_constants
    }

    pub fn on_event(&mut self, event: UserEvent) -> bool {
        match event {
            UserEvent::KeyDown { key_code, .. } => {
                *self = match key_code {
                    29 /* 0 */ => Self::DEFAULT,
                    18 /* 1 */ => Self::ONLY_NORMALS,
                    19 /* 2 */ => Self::HAS_AMBIENT,
                    20 /* 3 */ => Self::HAS_AMBIENT | Self::HAS_DIFFUSE,
                    21 /* 4 */ => Self::HAS_SPECULAR,
                    _ => return false
                };
            }
            _ => return false,
        }
        return true;
    }
}
