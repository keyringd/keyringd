mod named_pipe_handler;
mod stdio_handler;
mod user_only_security_descriptor;
mod windows_named_pipe_listener;

pub(crate) use git_bash_handler::GitBashHandler;
pub use named_pipe_handler::NamedPipeHandler;
pub(crate) use stdio_handler::StdioHandler;
mod git_bash_handler;
mod git_bash_socket_file;
mod git_bash_socket_listener;

#[cfg(test)]
mod test_support;

#[cfg(test)]
pub(crate) use test_support::assert_user_only_access;
#[cfg(test)]
pub(crate) use windows_named_pipe_listener::WindowsNamedPipeListener;
