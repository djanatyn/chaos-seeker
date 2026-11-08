use chaos_seeker::state::GameState;

use std::{thread::sleep, time::Duration};

use tetanes_core::prelude::*;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::WARN)
        .init();

    let _ = std::fs::create_dir("screenshots");

    // TODO: move control deck creation into GameState::new
    let mut control_deck = ControlDeck::new();
    control_deck.load_rom_path("ff1.nes")?;

    let mut game = GameState::new(control_deck);

    // while game.is_running() {
    //     let _ = game.tick();
    //     sleep(Duration::from_millis(50));
    // }

    Ok(())
}
