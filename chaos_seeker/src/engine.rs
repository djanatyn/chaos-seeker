use crate::{
    handlers::{char_create, intro, menu},
    state::{GameProgress, GameState, Handler, HandlerOutput},
};

impl GameState {
    /// Retrieve current handler based on game progress.
    fn handler(&self) -> Handler {
        match self.progress {
            GameProgress::BootIntro { .. } => intro::handler,
            GameProgress::BootMenu { .. } => menu::handler,
            GameProgress::CharacterCreation { .. } => char_create::handler,
            GameProgress::Pause | GameProgress::Unknown => |_| HandlerOutput {
                progress: GameProgress::Unknown,
                ..
            },
        }
    }

    /// Evaluate current handler, returning its output with advancing to the next frame.
    pub fn evaluate_handler(&mut self) -> HandlerOutput {
        self.handler()(self)
    }

    /// Apply a handler's output to advance to the next frame.
    pub fn apply_handler_output(&mut self, output: HandlerOutput) -> anyhow::Result<u64> {
        self.progress = output.progress;
        self.advance_frame(output.buttons)
    }

    /// Run current handler and advance the frame with handler output.
    pub fn auto_advance(&mut self) -> anyhow::Result<u64> {
        let output = self.handler()(self);
        self.apply_handler_output(output)
    }
}
