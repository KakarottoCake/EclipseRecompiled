// XInput backend for Windows (Xbox controllers)
#[cfg(target_os = "windows")]
use crate::input::backends::{Backend, ControllerInfo, RawInput};
#[cfg(target_os = "windows")]
use anyhow::Result;

#[cfg(target_os = "windows")]
pub struct XInputBackend;

#[cfg(target_os = "windows")]
impl XInputBackend {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

#[cfg(target_os = "windows")]
impl Backend for XInputBackend {
    fn name(&self) -> &'static str {
        "XInput"
    }

    fn update(&mut self) -> Result<()> {
        // XInput state is queried on-demand
        Ok(())
    }

    fn enumerate_controllers(&mut self) -> Result<Vec<ControllerInfo>> {
        // SDL owns XInput devices. Returning synthetic controllers here caused
        // four disconnected pads to appear on every Windows system.
        Ok(Vec::new())
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        if controller_id >= 4 {
            anyhow::bail!("Invalid XInput controller ID: {}", controller_id);
        }

        // In a real implementation, would query XInput state
        // For now, return empty input
        Ok(RawInput {
            buttons: vec![false; 10],
            axes: vec![0.0; 6],
            triggers: vec![0.0; 2],
            hat: None,
        })
    }
}
