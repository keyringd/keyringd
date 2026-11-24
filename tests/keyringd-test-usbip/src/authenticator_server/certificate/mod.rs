//! A disposable test CA and PEM server identity generated in Rust with rcgen.
//!
//! Entry point: [`HybridTestCertificate::generate`].

mod hybrid_test_certificate;
pub use hybrid_test_certificate::HybridTestCertificate;
