use image::{ImageBuffer, Rgba};
use tetanes_core::{input::JoypadBtnState, mem::Read, prelude::*};

/// Progress through Final Fantasy 1.
#[derive(Debug)]
pub enum GameProgress {
    /// Stuck in the intro.
    BootIntro,
    /// Main menu after intro.
    BootMenu,
    /// Character creation for new game.
    CharacterCreation,
    /// Unimplemented.
    Unknown,
}

/// State of the game.
#[derive(Debug)]
pub struct GameState {
    pub frame: u64,
    pub progress: GameProgress,
    control_deck: ControlDeck,
}

impl GameState {
    pub fn new(control_deck: ControlDeck) -> Self {
        Self {
            frame: 0,
            progress: GameProgress::BootIntro,
            control_deck,
        }
    }

    pub fn is_running(&self) -> bool {
        self.control_deck.is_running()
    }

    pub fn set_button(&mut self, btn: JoypadBtnState, state: bool) {
        self.control_deck
            .joypad_mut(Player::One)
            .set_button(btn, state)
    }

    pub fn image(&mut self) -> Option<ImageBuffer<Rgba<u8>, &[u8]>> {
        ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            Ppu::WIDTH,
            Ppu::HEIGHT,
            self.control_deck.frame_buffer(),
        )
    }
    /// Take a screenshot of the game.
    pub fn screenshot(&mut self, filename: &str) {
        // Process frame buffer (e.g. by rendering it to the screen)
        let image = ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            Ppu::WIDTH,
            Ppu::HEIGHT,
            self.control_deck.frame_buffer(),
        );

        if let Some(image) = image {
            let _ = image.save(&filename);
            // tracing::warn!(filename, "saved screenshot");
        }
    }

    /// Toggle the state of a button.
    pub fn toggle(&mut self, button: JoypadBtnState) {
        let p1 = self.control_deck.joypad_mut(Player::One);
        p1.set_button(button, !p1.button(button));
    }

    /// Release a button.
    pub fn _release(&mut self, button: JoypadBtnState) {
        self.control_deck
            .joypad_mut(Player::One)
            .set_button(button, false);
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.control_deck.bus_mut().read(addr)
    }

    /// Take a screenshot using a name based on the current frame.
    fn _snap(&mut self) {
        let filename = format!("screenshots/{:04}.png", self.frame);
        self.screenshot(&filename);
    }

    pub fn tick(&mut self) -> anyhow::Result<u64> {
        let time = self.control_deck.clock_frame()?;
        self.frame += 1;

        self.progress = self.handler();
        Ok(time)
    }
}
