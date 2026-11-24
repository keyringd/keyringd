mod pin_prompt_view;
#[allow(clippy::module_inception)]
mod pin_prompt_window;
pub(in crate::app::ui) use pin_prompt_view::PinPromptView;
pub(crate) use pin_prompt_window::PinPromptWindow;
