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
            // tracing::warn!(filename, "saved screenshot");
        }
    }

    /// Toggle the state of a button.
    pub fn toggle(control_deck: &mut ControlDeck, button: JoypadBtnState) {
        let p1 = control_deck.joypad_mut(Player::One);
        p1.set_button(button, !p1.button(button));
    }

    /// Release a button.
    pub fn _release(control_deck: &mut ControlDeck, button: JoypadBtnState) {
        control_deck
            .joypad_mut(Player::One)
            .set_button(button, false);
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
        bus.read(char_create::SLOT_SELECTED_ADDRESS) == 0x0
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

    pub const SLOT_SELECTED_ADDRESS: u16 = 0x67;
    pub const CLASS_SELECTED_ADDRESS: u16 = 0x300;

    #[derive(Debug)]
    pub enum Mode {
        SelectName,
        SelectClass,
    }

    impl Mode {
        fn current(bus: &mut Bus) -> Option<Self> {
            match bus.read(MODE_ADDRESS) {
                MODE_CLASS_SELECT_VALUE => Some(Mode::SelectClass),
                MODE_NAME_SELECT_VALUE => Some(Mode::SelectName),
                _ => None,
            }
        }
    }

    pub const MODE_ADDRESS: u16 = 0x1a;
    pub const MODE_CLASS_SELECT_VALUE: u8 = 0x8;
    pub const MODE_NAME_SELECT_VALUE: u8 = 0x4;

    pub const PARTY_COMPLETE_ADDRESS: u16 = 0x11;
    pub const PARTY_COMPLETE_VALUE: u8 = 0x0;

    #[derive(Debug, PartialEq)]
    pub enum StartingClass {
        Fighter,
        Thief,
        BlackBelt,
        RedMage,
        WhiteMage,
        BlackMage,
    }

    use StartingClass::*;

    pub const STARTING_PARTY: [(StartingClass, &str); 4] = [
        (Fighter, "LDA"),
        (BlackBelt, "ADC"),
        (WhiteMage, "JMP"),
        (BlackMage, "CMP"),
    ];

    impl StartingClass {
        fn selection(value: u8) -> Option<StartingClass> {
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

        /// Current party member slot selected class.
        fn current_class(bus: &mut Bus) -> Option<Self> {
            Self::selection(bus.read(CLASS_SELECTED_ADDRESS))
        }
    }

    pub type Character = char;
    pub type Coordinate = u16;
    pub type Value = u16;

    // TODO: 70
    pub const CHARACTER_MAP: [(Character, (Coordinate, Value)); 11] = [
        ('A', (0x0000, 0x8A)),
        ('B', (0x0001, 0x8B)),
        ('C', (0x0002, 0x8C)),
        ('D', (0x0003, 0x8D)),
        ('E', (0x0004, 0x8E)),
        ('F', (0x0005, 0x8F)),
        ('G', (0x0006, 0x90)),
        ('H', (0x0007, 0x91)),
        ('I', (0x0008, 0x92)),
        ('J', (0x0009, 0x93)),
        ('K', (0x0100, 0x94)),
    ];

    /// Current party member slot (0x0 -> 0x4)
    fn current_slot(bus: &mut Bus) -> Option<u8> {
        let value = bus.read(SLOT_SELECTED_ADDRESS);
        (0..4).contains(&value).then_some(value)
    }

    fn party_complete(bus: &mut Bus) -> bool {
        bus.read(PARTY_COMPLETE_ADDRESS) == PARTY_COMPLETE_VALUE
    }

    pub fn handler(_state: &GameState, control_deck: &mut ControlDeck) -> GameProgress {
        if party_complete(control_deck.bus_mut()) {
            util::toggle(control_deck, JoypadBtnState::START);
            return GameProgress::Unknown;
        }

        match Mode::current(control_deck.bus_mut()) {
            Some(Mode::SelectName) => {
                let current_class = StartingClass::current_class(control_deck.bus_mut())
                    .expect("failed to find current class");
                dbg!(current_class);
            }
            Some(Mode::SelectClass) => {
                let slot =
                    current_slot(control_deck.bus_mut()).expect("failed to identify party slot");
                let (desired_class, desired_name) = &STARTING_PARTY[slot as usize];
                let current_class = StartingClass::current_class(control_deck.bus_mut())
                    .expect("failed to find current class");

                if current_class == *desired_class {
                    util::toggle(control_deck, JoypadBtnState::A);
                } else {
                    util::toggle(control_deck, JoypadBtnState::B);
                }

                dbg!(slot, desired_class, desired_name, current_class);
            }
            None => panic!("invalid character creation mode"),
        }

        GameProgress::CharacterCreation
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
