use std::{thread::sleep, time::Duration};

use tetanes_core::{bus::Bus, input::JoypadBtnState, mem::Read, prelude::*};
use tracing::{self, instrument};

mod util {
    use super::*;

    /// Take a screenshot of the game.
    pub fn screenshot(control_deck: &mut ControlDeck, filename: &str) {
        // Process frame buffer (e.g. by rendering it to the screen)
        let image = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            Ppu::WIDTH,
            Ppu::HEIGHT,
            control_deck.frame_buffer(),
        );

        if let Some(image) = image {
            let _ = image.save(&filename);
            tracing::warn!(filename, "saved screenshot");
        }
    }

    /// Toggle the state of a button.
    pub fn toggle(control_deck: &mut ControlDeck, button: JoypadBtnState) {
        let p1 = control_deck.joypad_mut(Player::One);
        p1.set_button(button, !p1.button(button));
    }
}

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
}

impl GameState {
    /// Take a screenshot using a name based on the current frame.
    fn screenshot(&self, control_deck: &mut ControlDeck) {
        let filename = format!("screenshots/{:04}.png", self.frame);
        util::screenshot(control_deck, &filename);
    }

    /// Advance game state.
    fn handler(&self, control_deck: &mut ControlDeck) -> GameProgress {
        let task = match self.progress {
            GameProgress::BootIntro => intro::handler,
            GameProgress::BootMenu => menu::handler,
            GameProgress::CharacterCreation => char_create::handler,
            GameProgress::Unknown => todo!(),
        };

        task(&self, control_deck)
    }
}

mod intro {
    //! Intro cutscene handler.
    //!
    //! We wait a few frames before hitting the start button.

    use super::*;

    /// How long to wait before pressing buttons.
    pub const WAIT_FRAMES: u64 = 20;
    /// Address to check to see if we're at the main menu.
    pub const INTRO_PROGRESS_ADDRESS: u16 = 0x26;
    /// Value we expect to see if we're at the main menu.
    pub const INTRO_COMPLETE_VALUE: u8 = 0x10;

    #[instrument]
    fn at_main_menu(bus: &mut Bus) -> bool {
        bus.read(INTRO_PROGRESS_ADDRESS) == INTRO_COMPLETE_VALUE
    }

    /// Wait until the cutscene has played a bit, then hit start until we reach the main menu.
    #[instrument]
    pub fn handler(state: &GameState, control_deck: &mut ControlDeck) -> GameProgress {
        // wait for intro to play before hitting buttons
        if state.frame <= WAIT_FRAMES {
            return GameProgress::BootIntro;
        }

        if at_main_menu(control_deck.bus_mut()) {
            tracing::warn!(state.frame, "reached menu");
            let _ = util::screenshot(control_deck, "menu.png");
            control_deck
                .joypad_mut(Player::One)
                .set_button(JoypadBtnState::START, false);

            return GameProgress::BootMenu;
        }

        // toggle start button until we reach the main menu
        util::toggle(control_deck, JoypadBtnState::START);
        return GameProgress::BootIntro;
    }
}

mod menu {
    use super::*;

    /// Address showing cursor position in menu.
    ///
    /// Cursor can point at CONTINUE, or NEW GAME.
    pub const CURSOR_ADDRESS: u16 = 0x62;
    /// Value of CURSOR_ADDRESS when pointing at NEW GAME.
    pub const NEW_GAME_VALUE: u8 = 0x1;

    fn cursor_new_game(bus: &mut Bus) -> bool {
        bus.read(CURSOR_ADDRESS) == NEW_GAME_VALUE
    }

    fn at_character_creation(bus: &mut Bus) -> bool {
        bus.read(char_create::CHARACTER_SELECTED_ADDRESS) == 0x0
    }

    /// Move cursor to new game and hit start.
    #[instrument]
    pub fn handler(state: &GameState, control_deck: &mut ControlDeck) -> GameProgress {
        if at_character_creation(control_deck.bus_mut()) {
            tracing::warn!(state.frame, "reached char creation");
            return GameProgress::CharacterCreation;
        }

        if cursor_new_game(control_deck.bus_mut()) {
            // hit start for new game
            util::toggle(control_deck, JoypadBtnState::START);
        } else {
            // move cursor down to new game
            util::toggle(control_deck, JoypadBtnState::DOWN);
        }

        return GameProgress::BootMenu;
    }
}

mod char_create {
    use super::*;

    pub const CHARACTER_SELECTED_ADDRESS: u16 = 0x67;

    pub enum StartingClass {
        Fighter,
        Thief,
        BlackBelt,
        RedMage,
        WhiteMage,
        BlackMage,
    }

    impl StartingClass {
        fn selection(value: u8) -> Option<StartingClass> {
            use StartingClass::*;
            match value {
                0x0 => Some(Fighter),
                0x1 => Some(Thief),
                0x2 => Some(BlackBelt),
                0x3 => Some(RedMage),
                0x4 => Some(WhiteMage),
                0x5 => Some(BlackMage),
                _ => None,
            }
        }

        fn current_class(bus: &mut Bus) -> Option<StartingClass> {
            Self::selection(bus.read(CHARACTER_SELECTED_ADDRESS))
        }
    }

    pub fn handler(state: &GameState, control_deck: &mut ControlDeck) -> GameProgress {
        util::screenshot(control_deck, "char_creation.png");
        todo!()
    }
}

#[instrument]
pub fn play(state: &GameState, control_deck: &mut ControlDeck) -> GameProgress {
    state.screenshot(control_deck);
    state.handler(control_deck)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::WARN)
        .init();

    let _ = std::fs::create_dir("screenshots");

    let mut control_deck = ControlDeck::new();
    control_deck.load_rom_path("ff1.nes")?;

    let mut state = GameState {
        frame: 0,
        progress: GameProgress::BootIntro,
    };

    while control_deck.is_running() {
        control_deck.clock_frame()?;
        state.frame += 1;

        state.progress = play(&state, &mut control_deck);
        sleep(Duration::from_millis(50));
    }

    Ok(())
}
