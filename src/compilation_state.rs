use std::default::Default;

pub enum State {
    NotCompiling,
    Compiling
}

pub struct CompilationState {
    cur_state: State,
    pub message: String,
    pub progress: i32,
}

impl Default for CompilationState {
    fn default() -> Self {
        Self { cur_state: State::NotCompiling, message: String::new(), progress: 0 }
    }
}

impl CompilationState {
    pub fn compiling(message: String, progress: i32) -> Self {
        Self { cur_state: State::Compiling, message, progress }
    }
}
