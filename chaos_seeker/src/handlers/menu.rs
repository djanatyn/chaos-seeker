use crate::state::{BootMenuState, CharacterCreationState, GameProgress, GameState, HandlerOutput};

use tetanes_core::input::JoypadBtnState;
use tracing::{self, instrument};

/// Address showing cursor position in menu.
///
/// Cursor can point at CONTINUE, or NEW GAME.
pub const CURSOR_ADDRESS: u16 = 0x62;
/// Value of CURSOR_ADDRESS when pointing at NEW GAME.
pub const NEW_GAME_VALUE: u8 = 0x1;

pub const CHAR_CREATION_MODE_ADDRESS: u16 = 0x67;
pub const CHAR_CREATION_MODE_VALUE: u8 = 0x0;

fn cursor_new_game(state: &mut GameState) -> bool {
    state.read(CURSOR_ADDRESS) == NEW_GAME_VALUE
}

fn at_character_creation(state: &mut GameState) -> bool {
    state.read(CHAR_CREATION_MODE_ADDRESS) == CHAR_CREATION_MODE_VALUE
}

/// Move cursor to new game and hit start.
#[instrument]
pub fn handler(state: &mut GameState) -> HandlerOutput {
    if at_character_creation(state) {
        tracing::warn!(state.frame, "reached char creation");
        return HandlerOutput {
            progress: GameProgress::CharacterCreation(CharacterCreationState { .. }),
            ..
        };
    }

    let progress = GameProgress::BootMenu(BootMenuState { .. });
    if cursor_new_game(state) {
        // hit start for new game
        HandlerOutput {
            progress,
            buttons: JoypadBtnState::START,
        }
    } else {
        // move cursor down to new game
        HandlerOutput {
            progress,
            buttons: JoypadBtnState::DOWN,
        }
    }
}
