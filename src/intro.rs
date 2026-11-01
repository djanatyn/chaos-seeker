//! Intro cutscene handler.
//!
//! We wait a few frames before hitting the start button.

use crate::state::{GameProgress, GameState};
use tetanes_core::{input::JoypadBtnState, mem::Read};
use tracing::{self, instrument};

/// How long to wait before pressing buttons.
pub const WAIT_FRAMES: u64 = 20;
/// Address to check to see if we're at the main menu.
pub const INTRO_PROGRESS_ADDRESS: u16 = 0x26;
/// Value we expect to see if we're at the main menu.
pub const INTRO_COMPLETE_VALUE: u8 = 0x10;

#[instrument]
fn at_main_menu(state: &mut GameState) -> bool {
    state.read(INTRO_PROGRESS_ADDRESS) == INTRO_COMPLETE_VALUE
}

/// Wait until the cutscene has played a bit, then hit start until we reach the main menu.
#[instrument]
pub fn handler(state: &mut GameState) -> GameProgress {
    // wait for intro to play before hitting buttons
    if state.frame <= WAIT_FRAMES {
        return GameProgress::BootIntro;
    }

    if at_main_menu(state) {
        tracing::warn!(state.frame, "reached menu");
        state.screenshot("menu.png");
        state.set_button(JoypadBtnState::START, false);

        return GameProgress::BootMenu;
    }

    // toggle start button until we reach the main menu
    state.toggle(JoypadBtnState::START);
    GameProgress::BootIntro
}
