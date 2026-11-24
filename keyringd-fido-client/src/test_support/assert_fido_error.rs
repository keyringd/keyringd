macro_rules! assert_fido_error {
    ($result:expr, $pattern:pat $(,)?) => {
        match $result {
            Err(error) => assert!(matches!(&error, $pattern), "unexpected error: {error:?}"),
            Ok(_) => panic!("expected a FIDO error"),
        }
    };
}

pub(crate) use assert_fido_error;
