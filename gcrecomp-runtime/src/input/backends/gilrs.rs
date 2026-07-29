// Gilrs backend for cross-platform gamepad support
use crate::input::backends::{Backend, ControllerInfo, ControllerType, RawInput};
use anyhow::Result;
use gilrs::{Axis, Gilrs};

pub struct GilrsBackend {
    gilrs: Gilrs,
}

impl GilrsBackend {
    pub fn new() -> Result<Self> {
        let gilrs =
            Gilrs::new().map_err(|e| anyhow::anyhow!("Failed to initialize gilrs: {}", e))?;

        Ok(Self { gilrs })
    }
}

impl Backend for GilrsBackend {
    fn name(&self) -> &'static str {
        "gilrs"
    }

    fn update(&mut self) -> Result<()> {
        // Process events
        while self.gilrs.next_event().is_some() {}
        Ok(())
    }

    fn enumerate_controllers(&mut self) -> Result<Vec<ControllerInfo>> {
        let mut controllers = Vec::new();

        for (id, gamepad) in self.gilrs.gamepads() {
            let name = gamepad.name();
            let controller_type = detect_controller_type(name);

            controllers.push(ControllerInfo {
                id: id.into(),
                name: name.to_string(),
                controller_type,
                button_count: 16, // Standard gamepad button count
                axis_count: 6,    // Standard gamepad axis count
            });
        }

        Ok(controllers)
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        // Find gamepad by iterating gamepads (gilrs 0.10 API)
        let gamepad = self
            .gilrs
            .gamepads()
            .find(|(id, _)| usize::from(*id) == controller_id)
            .map(|(_, g)| g);

        if let Some(gamepad) = gamepad {
            // Keep this canonical order in sync with the SDL backend:
            // south, east, west, north, select, guide, start, stick clicks,
            // shoulders, and D-pad.
            use gilrs::Button;
            let buttons = vec![
                gamepad.is_pressed(Button::South),
                gamepad.is_pressed(Button::East),
                gamepad.is_pressed(Button::West),
                gamepad.is_pressed(Button::North),
                gamepad.is_pressed(Button::Select),
                gamepad.is_pressed(Button::Mode),
                gamepad.is_pressed(Button::Start),
                gamepad.is_pressed(Button::LeftThumb),
                gamepad.is_pressed(Button::RightThumb),
                gamepad.is_pressed(Button::LeftTrigger),
                gamepad.is_pressed(Button::RightTrigger),
                gamepad.is_pressed(Button::DPadUp),
                gamepad.is_pressed(Button::DPadDown),
                gamepad.is_pressed(Button::DPadLeft),
                gamepad.is_pressed(Button::DPadRight),
                false,
            ];

            // Read axes explicitly
            let axes = vec![
                gamepad.value(Axis::LeftStickX),
                gamepad.value(Axis::LeftStickY),
                gamepad.value(Axis::RightStickX),
                gamepad.value(Axis::RightStickY),
            ];

            // Read triggers
            let left_trigger = gamepad.value(Axis::LeftZ);
            let right_trigger = gamepad.value(Axis::RightZ);
            let triggers = vec![
                normalize_trigger(left_trigger),
                normalize_trigger(right_trigger),
            ];

            Ok(RawInput {
                buttons,
                axes,
                triggers,
                hat: None,
            })
        } else {
            anyhow::bail!("Controller not found: {}", controller_id);
        }
    }
}

fn normalize_trigger(value: f32) -> f32 {
    if value < 0.0 {
        ((value + 1.0) * 0.5).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn detect_controller_type(name: &str) -> ControllerType {
    let name_lower = name.to_lowercase();
    if name_lower.contains("xbox") {
        ControllerType::Xbox
    } else if name_lower.contains("playstation") || name_lower.contains("dualshock") {
        ControllerType::PlayStation
    } else if name_lower.contains("switch") || name_lower.contains("pro controller") {
        ControllerType::SwitchPro
    } else {
        ControllerType::Generic
    }
}
