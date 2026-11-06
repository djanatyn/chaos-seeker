use crate::{
    handlers::{char_create, intro, menu},
    state::{ButtonSet, GameProgress, GameState, HandlerOutput},
};

// pub type Handler = fn(&mut GameState) -> HandlerOutput;
pub type Handler = fn(&mut GameState) -> GameProgress;

impl GameState {
    fn handler(&self) -> Handler {
        match self.progress {
            GameProgress::BootIntro { .. } => intro::handler,
            GameProgress::BootMenu { .. } => menu::handler,
            GameProgress::CharacterCreation { .. } => char_create::handler,
            GameProgress::Pause | GameProgress::Unknown => |_| GameProgress::Pause,
            // GameProgress::Pause | GameProgress::Unknown => |_| HandlerOutput {
            //     progress: GameProgress::Pause,
            //     buttons: ButtonSet {},
            // },
        }
    }

    /// Advance game state.
    // pub fn progress(&mut self) -> HandlerOutput {
    pub fn advance_progress(&mut self) -> GameProgress {
        self.handler()(self)
    }
}
