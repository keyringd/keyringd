#[allow(clippy::module_inception)]
mod pin_prompt_window;
pub(in crate::app) use pin_prompt_window::PinPromptWindow;
mod pin_prompt_view;
pub(in crate::app::ui) use pin_prompt_view::PinPromptView;
