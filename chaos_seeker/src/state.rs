use image::{ImageBuffer, Rgba};
use tetanes_core::{input::JoypadBtnState, mem::Read, prelude::*};

use crate::handlers;

pub type Handler = fn(&mut GameState) -> HandlerOutput;

#[derive(Debug)]
pub struct HandlerOutput {
    pub progress: GameProgress,
    pub buttons: JoypadBtnState = JoypadBtnState::empty(),
}

/// Progress through Final Fantasy 1.
#[derive(Debug, Clone, PartialEq)]
pub enum GameProgress {
    /// Stuck in the intro.
    BootIntro(BootIntroState),
    /// Main menu after intro.
    BootMenu(BootMenuState),
    /// Character creation for new game.
    CharacterCreation(CharacterCreationState),
    /// Intentionally paused.
    Pause,
    /// Unimplemented.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootIntroState {
    pub remaining_frames: u64 = handlers::intro::WAIT_FRAMES,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BootMenuState {
    pub cursor_value: Option<u8> = None,
    pub hit_start: bool = false,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterCreationState {
    pub party_complete: bool = false,
    pub mode: Option<handlers::char_create::Mode> = None,
    pub current_class: Option<handlers::char_create::StartingClass> = None,
    pub slot: Option<handlers::char_create::Slot> = None,
    pub current_name: Option<String> = None,
    pub cursor_coordinate: Option<handlers::char_create::NameCoordinate> = None,
    pub target_cursor_coordinate: Option<handlers::char_create::NameCoordinate> = None,
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
            progress: GameProgress::BootIntro(BootIntroState { .. }),
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
        let image = ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            Ppu::WIDTH,
            Ppu::HEIGHT,
            self.control_deck.frame_buffer(),
        )?;

        image.save("frame.png").ok()?;
        Some(image)
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
            let _ = image.save(filename);
            // tracing::warn!(filename, "saved screenshot");
        }
    }

    pub fn buttons(&mut self) -> JoypadBtnState {
        self.control_deck.joypad(Player::One).buttons
    }

    pub fn tap(&mut self, button: JoypadBtnState) -> JoypadBtnState {
        let p1 = self.control_deck.joypad_mut(Player::One);

        if p1.buttons.contains(button) {
            JoypadBtnState::empty()
        } else {
            button
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

    pub fn advance_frame(&mut self, inputs: JoypadBtnState) -> anyhow::Result<u64> {
        self.control_deck.joypad_mut(Player::One).buttons = inputs;
        let time = self.control_deck.clock_frame()?;
        self.frame += 1;
        Ok(time)
    }

    // pub fn tick(&mut self) -> anyhow::Result<u64> {
    //     let time = self.control_deck.clock_frame()?;
    //     self.frame += 1;

    //     self.progress = self.advance_progress();
    //     Ok(time)
    // }
}
