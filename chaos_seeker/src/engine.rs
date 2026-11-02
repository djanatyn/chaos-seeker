use crate::{
    handlers::char_create,
    handlers::intro,
    handlers::menu,
    state::{GameProgress, GameState},
};

use tetanes_core::{input::JoypadBtnState, mem::Read, prelude::*};

impl GameState {
    /// Advance game state.
    pub fn handler(&mut self) -> GameProgress {
        let task = match self.progress {
            GameProgress::BootIntro => intro::handler,
            GameProgress::BootMenu => menu::handler,
            GameProgress::CharacterCreation => char_create::handler,
            GameProgress::Unknown => todo!(),
        };

        task(self)
    }
}
