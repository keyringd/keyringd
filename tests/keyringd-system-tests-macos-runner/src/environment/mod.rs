//! Private workspace paths and virtual authenticator server lifecycle.
mod authenticator_environment;
mod workspace;

pub(crate) use authenticator_environment::AuthenticatorEnvironment;
pub(crate) use workspace::Workspace;
