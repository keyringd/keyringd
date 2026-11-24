mod unix_socket_handler;

#[cfg(all(test, feature = "test-support"))]
mod tests;

pub(crate) use unix_socket_handler::UnixSocketHandler;
