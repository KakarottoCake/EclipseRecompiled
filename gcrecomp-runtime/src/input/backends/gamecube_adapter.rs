//! Native support for Wii U / Switch GameCube controller USB adapters.
//!
//! The official Nintendo adapter and compatible adapters in Wii U mode expose
//! four controller ports over two interrupt endpoints. On Windows, the adapter
//! must use a libusb-compatible driver such as WinUSB.

use crate::input::backends::{Backend, ControllerInfo, ControllerType, HatState, RawInput};
use anyhow::{Context, Result};
use rusb::{DeviceHandle, GlobalContext};
use std::time::{Duration, Instant};

const INTERFACE: u8 = 0;
const INPUT_ENDPOINT: u8 = 0x81;
const OUTPUT_ENDPOINT: u8 = 0x02;
const INPUT_REPORT: u8 = 0x21;
const INIT_REPORT: u8 = 0x13;
const RUMBLE_REPORT: u8 = 0x11;
const REPORT_LEN: usize = 37;
const PORT_REPORT_LEN: usize = 9;

const SUPPORTED_ADAPTERS: &[(u16, u16, &str)] = &[
    (0x057e, 0x0337, "Nintendo GameCube Adapter"),
    (0x0079, 0x1843, "Mayflash GameCube Adapter"),
];

pub struct GameCubeAdapterBackend {
    handle: Option<DeviceHandle<GlobalContext>>,
    adapter_name: String,
    ports: [Option<RawInput>; 4],
    rumble: [bool; 4],
    last_connect_attempt: Option<Instant>,
}

impl GameCubeAdapterBackend {
    pub fn new() -> Result<Self> {
        let mut backend = Self {
            handle: None,
            adapter_name: "GameCube Adapter".to_string(),
            ports: std::array::from_fn(|_| None),
            rumble: [false; 4],
            last_connect_attempt: None,
        };
        backend.try_connect();
        Ok(backend)
    }

    fn try_connect(&mut self) {
        if self.handle.is_some() {
            return;
        }
        if self
            .last_connect_attempt
            .is_some_and(|last| last.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        self.last_connect_attempt = Some(Instant::now());

        for &(vendor, product, name) in SUPPORTED_ADAPTERS {
            let Some(handle) = rusb::open_device_with_vid_pid(vendor, product) else {
                continue;
            };

            let _ = handle.set_auto_detach_kernel_driver(true);
            if let Err(error) = handle.claim_interface(INTERFACE) {
                log::warn!(
                    "Found {name}, but could not claim it: {error}. \
                     On Windows, select the WinUSB driver for WUP-028."
                );
                continue;
            }

            if let Err(error) =
                handle.write_interrupt(OUTPUT_ENDPOINT, &[INIT_REPORT], Duration::from_secs(1))
            {
                log::warn!("Found {name}, but initialization failed: {error}");
                let _ = handle.release_interface(INTERFACE);
                continue;
            }

            log::info!("Connected to {name} ({vendor:04x}:{product:04x})");
            self.adapter_name = name.to_string();
            self.handle = Some(handle);
            return;
        }
    }

    fn disconnect(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.release_interface(INTERFACE);
        }
        self.ports = std::array::from_fn(|_| None);
    }

    fn read_reports(&mut self) -> Result<()> {
        let Some(handle) = self.handle.as_mut() else {
            return Ok(());
        };

        let mut report = [0u8; REPORT_LEN];
        loop {
            match handle.read_interrupt(INPUT_ENDPOINT, &mut report, Duration::from_millis(1)) {
                Ok(REPORT_LEN) if report[0] == INPUT_REPORT => {
                    self.ports = parse_report(&report);
                }
                Ok(_) => {}
                Err(rusb::Error::Timeout) => break,
                Err(rusb::Error::NoDevice) => {
                    self.disconnect();
                    break;
                }
                Err(error) => return Err(error).context("reading GameCube adapter input"),
            }
        }
        Ok(())
    }

    fn write_rumble(&mut self) -> Result<()> {
        let Some(handle) = self.handle.as_mut() else {
            return Ok(());
        };
        let report = [
            RUMBLE_REPORT,
            u8::from(self.rumble[0]),
            u8::from(self.rumble[1]),
            u8::from(self.rumble[2]),
            u8::from(self.rumble[3]),
        ];
        handle
            .write_interrupt(OUTPUT_ENDPOINT, &report, Duration::from_millis(100))
            .context("writing GameCube adapter rumble")?;
        Ok(())
    }
}

impl Backend for GameCubeAdapterBackend {
    fn name(&self) -> &'static str {
        "GameCube adapter"
    }

    fn update(&mut self) -> Result<()> {
        self.try_connect();
        self.read_reports()
    }

    fn enumerate_controllers(&mut self) -> Result<Vec<ControllerInfo>> {
        Ok(self
            .ports
            .iter()
            .enumerate()
            .filter_map(|(port, input)| {
                input.as_ref().map(|_| ControllerInfo {
                    id: port,
                    name: format!("{} — Port {}", self.adapter_name, port + 1),
                    controller_type: ControllerType::GameCubeAdapter,
                    button_count: 12,
                    axis_count: 6,
                })
            })
            .collect())
    }

    fn get_input(&self, controller_id: usize) -> Result<RawInput> {
        self.ports
            .get(controller_id)
            .and_then(Clone::clone)
            .with_context(|| format!("no controller in adapter port {}", controller_id + 1))
    }

    fn set_rumble(&mut self, controller_id: usize, enabled: bool) -> Result<()> {
        let rumble = self
            .rumble
            .get_mut(controller_id)
            .with_context(|| format!("invalid adapter port {}", controller_id + 1))?;
        if *rumble != enabled {
            *rumble = enabled;
            self.write_rumble()?;
        }
        Ok(())
    }
}

fn parse_report(report: &[u8; REPORT_LEN]) -> [Option<RawInput>; 4] {
    std::array::from_fn(|port| {
        let offset = 1 + port * PORT_REPORT_LEN;
        let status = report[offset];
        if status & 0x30 == 0 {
            return None;
        }

        let first = report[offset + 1];
        let second = report[offset + 2];
        let d_up = first & 0x80 != 0;
        let d_down = first & 0x40 != 0;
        let d_left = first & 0x10 != 0;
        let d_right = first & 0x20 != 0;

        Some(RawInput {
            buttons: vec![
                first & 0x01 != 0,  // A
                first & 0x02 != 0,  // B
                first & 0x04 != 0,  // X
                first & 0x08 != 0,  // Y
                second & 0x01 != 0, // Start
                d_up,
                d_down,
                d_left,
                d_right,
                second & 0x08 != 0, // L digital
                second & 0x04 != 0, // R digital
                second & 0x02 != 0, // Z
            ],
            axes: vec![
                normalize_axis(report[offset + 3]),
                normalize_axis(report[offset + 4]),
                normalize_axis(report[offset + 5]),
                normalize_axis(report[offset + 6]),
            ],
            triggers: vec![
                report[offset + 7] as f32 / 255.0,
                report[offset + 8] as f32 / 255.0,
            ],
            hat: Some(HatState {
                up: d_up,
                down: d_down,
                left: d_left,
                right: d_right,
            }),
        })
    })
}

fn normalize_axis(value: u8) -> f32 {
    let centered = value as i16 - 128;
    if centered >= 0 {
        centered as f32 / 127.0
    } else {
        centered as f32 / 128.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_connected_controller_report() {
        let mut report = [0u8; REPORT_LEN];
        report[0] = INPUT_REPORT;
        report[1] = 0x10;
        report[2] = 0x81; // A + D-pad up
        report[3] = 0x06; // Z + R digital
        report[4] = 255;
        report[5] = 0;
        report[6] = 128;
        report[7] = 128;
        report[8] = 64;
        report[9] = 255;

        let ports = parse_report(&report);
        let input = ports[0].as_ref().expect("port 1 connected");
        assert!(input.buttons[0]);
        assert!(input.buttons[5]);
        assert!(input.buttons[10]);
        assert!(input.buttons[11]);
        assert_eq!(input.axes[0], 1.0);
        assert_eq!(input.axes[1], -1.0);
        assert_eq!(input.triggers[1], 1.0);
        assert!(ports[1].is_none());
    }
}
