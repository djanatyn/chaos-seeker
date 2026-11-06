use crate::{
    handlers::char_create,
    handlers::intro,
    handlers::menu,
    state::{GameProgress, GameState},
};

impl GameState {
    /// Advance game state.
    pub fn handler(&mut self) -> GameProgress {
        let task = match self.progress {
            GameProgress::BootIntro { .. } => intro::handler,
            GameProgress::BootMenu { .. } => menu::handler,
            GameProgress::CharacterCreation { .. } => char_create::handler,
            GameProgress::Pause => return self.progress.clone(),
            GameProgress::Unknown => return self.progress.clone(),
        };

        task(self)
    }
}
