use crate::app::ui::agent_prompt_window::AgentPromptWindow;

pub struct AgentPromptWindowActivationSession {
    handle: usize,
}

impl AgentPromptWindowActivationSession {
    pub(super) fn new(handle: usize) -> Self {
        Self { handle }
    }
}

impl Drop for AgentPromptWindowActivationSession {
    fn drop(&mut self) {
        let window = AgentPromptWindow {
            handle: self.handle,
        };
        window.hide();
    }
}
