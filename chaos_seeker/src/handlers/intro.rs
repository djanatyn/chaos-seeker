//! Intro cutscene handler.
//!
//! We wait a few frames before hitting the start button.

use crate::state::{BootIntroState, BootMenuState, GameProgress, GameState, HandlerOutput};
use tetanes_core::input::JoypadBtnState;
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
pub fn handler(state: &mut GameState) -> HandlerOutput {
    let remaining_frames = WAIT_FRAMES.saturating_sub(state.frame);
    // wait for intro to play before hitting buttons
    if state.frame <= WAIT_FRAMES {
        return HandlerOutput {
            progress: GameProgress::BootIntro(BootIntroState { remaining_frames }),
            buttons: JoypadBtnState::empty(),
        };
    }

    if at_main_menu(state) {
        tracing::warn!(state.frame, "reached menu");
        state.screenshot("menu.png");

        return HandlerOutput {
            progress: GameProgress::BootMenu(BootMenuState { .. }),
            buttons: JoypadBtnState::empty(),
        };
    } else {
        // hold start button until we reach the main menu
        HandlerOutput {
            progress: GameProgress::BootIntro(BootIntroState { remaining_frames }),
            buttons: JoypadBtnState::START,
        }
    }
}
