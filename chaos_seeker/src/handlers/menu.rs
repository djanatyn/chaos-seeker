use crate::handlers::char_create;
use crate::state::{BootMenuState, CharacterCreationState, GameProgress, GameState};

use tetanes_core::{input::JoypadBtnState, mem::Read};
use tracing::{self, instrument};

/// Address showing cursor position in menu.
///
/// Cursor can point at CONTINUE, or NEW GAME.
pub const CURSOR_ADDRESS: u16 = 0x62;
/// Value of CURSOR_ADDRESS when pointing at NEW GAME.
pub const NEW_GAME_VALUE: u8 = 0x1;

fn cursor_new_game(state: &mut GameState) -> bool {
    state.read(CURSOR_ADDRESS) == NEW_GAME_VALUE
}

fn at_character_creation(state: &mut GameState) -> bool {
    state.read(char_create::SLOT_SELECTED_ADDRESS) == 0x0
}

/// Move cursor to new game and hit start.
#[instrument]
pub fn handler(state: &mut GameState) -> GameProgress {
    if at_character_creation(state) {
        tracing::warn!(state.frame, "reached char creation");
        return GameProgress::CharacterCreation(CharacterCreationState { .. });
    }

    if cursor_new_game(state) {
        // hit start for new game
        state.toggle(JoypadBtnState::START);
    } else {
        // move cursor down to new game
        state.toggle(JoypadBtnState::DOWN);
    }

    GameProgress::BootMenu(BootMenuState { .. })
}
