//! Controller discovery, stable IDs, mapping, and routing.

use crate::input::backends::{Backend, ControllerInfo};
use crate::input::gamecube_mapping::GameCubeMapping;
use crate::input::profiles::ControllerProfile;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

struct BackendSlot {
    backend: Box<dyn Backend>,
}

pub struct ControllerManager {
    backends: Vec<BackendSlot>,
    controllers: HashMap<usize, ControllerState>,
    device_ids: HashMap<(usize, usize), usize>,
    gamecube_mappings: HashMap<usize, GameCubeMapping>,
    profiles: HashMap<String, ControllerProfile>,
    next_id: usize,
}

#[derive(Debug, Clone)]
pub struct ControllerState {
    pub id: usize,
    pub info: ControllerInfo,
    pub connected: bool,
    pub last_update: std::time::Instant,
    backend_index: usize,
    backend_controller_id: usize,
}

impl ControllerManager {
    pub fn new() -> Result<Self> {
        let mut backends = Vec::new();

        Self::add_backend(
            &mut backends,
            crate::input::backends::gamecube_adapter::GameCubeAdapterBackend::new(),
        );

        // SDL has the broadest consistent mapping database. Use gilrs only when
        // SDL cannot initialize, avoiding duplicate entries for every gamepad.
        let sdl = crate::input::backends::sdl2::SDL2Backend::new();
        if sdl.is_ok() {
            Self::add_backend(&mut backends, sdl);
        } else {
            Self::add_backend(
                &mut backends,
                crate::input::backends::gilrs::GilrsBackend::new(),
            );
        }

        if backends.is_empty() {
            anyhow::bail!("no controller backend could be initialized");
        }

        Ok(Self {
            backends,
            controllers: HashMap::new(),
            device_ids: HashMap::new(),
            gamecube_mappings: HashMap::new(),
            profiles: HashMap::new(),
            next_id: 0,
        })
    }

    fn add_backend<T: Backend + 'static>(backends: &mut Vec<BackendSlot>, backend: Result<T>) {
        match backend {
            Ok(backend) => {
                log::info!("Input backend ready: {}", backend.name());
                backends.push(BackendSlot {
                    backend: Box::new(backend),
                });
            }
            Err(error) => log::debug!("Input backend unavailable: {error}"),
        }
    }

    pub fn update(&mut self) -> Result<()> {
        let mut discovered = Vec::new();
        for (backend_index, slot) in self.backends.iter_mut().enumerate() {
            if let Err(error) = slot.backend.update() {
                log::warn!("{} input update failed: {error}", slot.backend.name());
                continue;
            }
            match slot.backend.enumerate_controllers() {
                Ok(controllers) => {
                    discovered.extend(controllers.into_iter().map(|info| (backend_index, info)));
                }
                Err(error) => {
                    log::warn!("{} enumeration failed: {error}", slot.backend.name());
                }
            }
        }

        let mut connected = HashSet::new();
        let mut newly_connected = Vec::new();

        for (backend_index, mut info) in discovered {
            let backend_controller_id = info.id;
            let key = (backend_index, backend_controller_id);
            let public_id = *self.device_ids.entry(key).or_insert_with(|| {
                let id = self.next_id;
                self.next_id += 1;
                id
            });
            connected.insert(public_id);
            info.id = public_id;

            if let Some(state) = self.controllers.get_mut(&public_id) {
                state.connected = true;
                state.info = info;
                state.last_update = std::time::Instant::now();
            } else {
                log::info!(
                    "Controller connected: {} [{}]",
                    info.name,
                    self.backends[backend_index].backend.name()
                );
                self.controllers.insert(
                    public_id,
                    ControllerState {
                        id: public_id,
                        info,
                        connected: true,
                        last_update: std::time::Instant::now(),
                        backend_index,
                        backend_controller_id,
                    },
                );
                newly_connected.push(public_id);
            }
        }

        self.controllers.retain(|id, state| {
            if connected.contains(id) {
                true
            } else {
                log::info!("Controller disconnected: {}", state.info.name);
                self.gamecube_mappings.remove(id);
                false
            }
        });

        for id in newly_connected {
            self.load_default_mapping(id)?;
        }
        Ok(())
    }

    pub fn get_controller_count(&self) -> usize {
        self.controllers.len()
    }

    pub fn controllers(&self) -> Vec<&ControllerState> {
        let mut controllers: Vec<_> = self.controllers.values().collect();
        controllers.sort_by_key(|state| state.id);
        controllers
    }

    pub fn get_controller_state(&self, id: usize) -> Option<&ControllerState> {
        self.controllers.get(&id)
    }

    pub fn get_gamecube_input(&self, controller_id: usize) -> Option<GameCubeInput> {
        let state = self.controllers.get(&controller_id)?;
        let mapping = self.gamecube_mappings.get(&controller_id)?;
        let input = self.backends[state.backend_index]
            .backend
            .get_input(state.backend_controller_id)
            .ok()?;
        Some(mapping.map_to_gamecube(&input))
    }

    pub fn set_rumble(&mut self, controller_id: usize, enabled: bool) -> Result<()> {
        let state = self
            .controllers
            .get(&controller_id)
            .with_context(|| format!("unknown controller {controller_id}"))?;
        self.backends[state.backend_index]
            .backend
            .set_rumble(state.backend_controller_id, enabled)
    }

    pub fn set_mapping(&mut self, controller_id: usize, mapping: GameCubeMapping) {
        self.gamecube_mappings.insert(controller_id, mapping);
    }

    pub fn load_profile(&mut self, controller_id: usize, profile_name: &str) -> Result<()> {
        if let Some(profile) = self.profiles.get(profile_name) {
            let mapping = profile.to_gamecube_mapping()?;
            self.set_mapping(controller_id, mapping);
        }
        Ok(())
    }

    pub fn save_profile(&mut self, name: String, controller_id: usize) -> Result<()> {
        if let Some(mapping) = self.gamecube_mappings.get(&controller_id) {
            let profile = ControllerProfile::from_mapping(name, mapping.clone());
            self.profiles.insert(profile.name.clone(), profile);
        }
        Ok(())
    }

    fn load_default_mapping(&mut self, controller_id: usize) -> Result<()> {
        if let Some(state) = self.controllers.get(&controller_id) {
            let default_mapping = GameCubeMapping::default_for_controller(&state.info)?;
            self.set_mapping(controller_id, default_mapping);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameCubeInput {
    pub buttons: GameCubeButtons,
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
    pub left_trigger: f32,
    pub right_trigger: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameCubeButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub start: bool,
    pub d_up: bool,
    pub d_down: bool,
    pub d_left: bool,
    pub d_right: bool,
    pub l: bool,
    pub r: bool,
    pub z: bool,
}
