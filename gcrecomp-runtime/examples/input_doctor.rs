use anyhow::Result;
use gcrecomp_runtime::input::ControllerManager;
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    println!("Watching for controllers for 10 seconds...");
    println!("Connect a pad or plug a controller into the GameCube adapter.");

    let mut manager = ControllerManager::new()?;
    let started = Instant::now();
    let mut announced = HashSet::new();

    while started.elapsed() < Duration::from_secs(10) {
        manager.update()?;
        for controller in manager.controllers() {
            if announced.insert(controller.id) {
                println!(
                    "[{}] {} ({:?})",
                    controller.id, controller.info.name, controller.info.controller_type
                );
            }
            if let Some(input) = manager.get_gamecube_input(controller.id) {
                if input != Default::default() {
                    println!("[{}] {:?}", controller.id, input);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    println!(
        "Detected {} connected controller(s).",
        manager.get_controller_count()
    );
    Ok(())
}
