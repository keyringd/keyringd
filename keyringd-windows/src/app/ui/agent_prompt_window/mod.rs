mod agent_prompt_thumbnail;
#[allow(clippy::module_inception)]
mod agent_prompt_window;
mod agent_prompt_window_activation_session;
use agent_prompt_thumbnail::AgentPromptThumbnail;
pub(crate) use agent_prompt_window::AgentPromptWindow;
pub use agent_prompt_window_activation_session::AgentPromptWindowActivationSession;
