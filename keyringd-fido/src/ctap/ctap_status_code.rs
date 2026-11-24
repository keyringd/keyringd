#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
pub enum CtapStatusCode {
    #[error("CTAP2_OK")]
    Ok,
    #[error("CTAP1_ERR_INVALID_COMMAND")]
    InvalidCommand,
    #[error("CTAP1_ERR_INVALID_PARAMETER")]
    InvalidParameter,
    #[error("CTAP1_ERR_INVALID_LENGTH")]
    InvalidLength,
    #[error("CTAP1_ERR_INVALID_SEQ")]
    InvalidSequence,
    #[error("CTAP1_ERR_TIMEOUT")]
    Timeout,
    #[error("CTAP1_ERR_CHANNEL_BUSY")]
    ChannelBusy,
    #[error("CTAP1_ERR_LOCK_REQUIRED")]
    LockRequired,
    #[error("CTAP1_ERR_INVALID_CHANNEL")]
    InvalidChannel,
    #[error("CTAP2_ERR_CBOR_UNEXPECTED_TYPE")]
    CborUnexpectedType,
    #[error("CTAP2_ERR_INVALID_CBOR")]
    InvalidCbor,
    #[error("CTAP2_ERR_MISSING_PARAMETER")]
    MissingParameter,
    #[error("CTAP2_ERR_LIMIT_EXCEEDED")]
    LimitExceeded,
    #[error("CTAP2_ERR_UNSUPPORTED_EXTENSION")]
    UnsupportedExtension,
    #[error("CTAP2_ERR_FP_DATABASE_FULL")]
    FingerprintDatabaseFull,
    #[error("CTAP2_ERR_LARGE_BLOB_STORAGE_FULL")]
    LargeBlobStorageFull,
    #[error("CTAP2_ERR_CREDENTIAL_EXCLUDED")]
    CredentialExcluded,
    #[error("CTAP2_ERR_PROCESSING")]
    Processing,
    #[error("CTAP2_ERR_INVALID_CREDENTIAL")]
    InvalidCredential,
    #[error("CTAP2_ERR_USER_ACTION_PENDING")]
    UserActionPending,
    #[error("CTAP2_ERR_OPERATION_PENDING")]
    OperationPending,
    #[error("CTAP2_ERR_NO_OPERATIONS")]
    NoOperations,
    #[error("CTAP2_ERR_UNSUPPORTED_ALGORITHM")]
    UnsupportedAlgorithm,
    #[error("CTAP2_ERR_OPERATION_DENIED")]
    OperationDenied,
    #[error("CTAP2_ERR_KEY_STORE_FULL")]
    KeyStoreFull,
    #[error("CTAP2_ERR_NO_OPERATION_PENDING")]
    NoOperationPending,
    #[error("CTAP2_ERR_UNSUPPORTED_OPTION")]
    UnsupportedOption,
    #[error("CTAP2_ERR_INVALID_OPTION")]
    InvalidOption,
    #[error("CTAP2_ERR_KEEPALIVE_CANCEL")]
    KeepaliveCancel,
    #[error("CTAP2_ERR_NO_CREDENTIALS")]
    NoCredentials,
    #[error("CTAP2_ERR_USER_ACTION_TIMEOUT")]
    UserActionTimeout,
    #[error("CTAP2_ERR_NOT_ALLOWED")]
    NotAllowed,
    #[error("CTAP2_ERR_PIN_INVALID")]
    PinInvalid,
    #[error("CTAP2_ERR_PIN_BLOCKED")]
    PinBlocked,
    #[error("CTAP2_ERR_PIN_AUTH_INVALID")]
    PinAuthInvalid,
    #[error("CTAP2_ERR_PIN_AUTH_BLOCKED")]
    PinAuthBlocked,
    #[error("CTAP2_ERR_PIN_NOT_SET")]
    PinNotSet,
    #[error("CTAP2_ERR_PUAT_REQUIRED")]
    PinUvAuthTokenRequired,
    #[error("CTAP2_ERR_PIN_POLICY_VIOLATION")]
    PinPolicyViolation,
    #[error("CTAP2_ERR_PIN_TOKEN_EXPIRED")]
    PinTokenExpired,
    #[error("CTAP2_ERR_REQUEST_TOO_LARGE")]
    RequestTooLarge,
    #[error("CTAP2_ERR_ACTION_TIMEOUT")]
    ActionTimeout,
    #[error("CTAP2_ERR_UP_REQUIRED")]
    UserPresenceRequired,
    #[error("CTAP2_ERR_UV_BLOCKED")]
    UserVerificationBlocked,
    #[error("CTAP2_ERR_INTEGRITY_FAILURE")]
    IntegrityFailure,
    #[error("CTAP2_ERR_INVALID_SUBCOMMAND")]
    InvalidSubcommand,
    #[error("CTAP2_ERR_UV_INVALID")]
    UserVerificationInvalid,
    #[error("CTAP2_ERR_UNAUTHORIZED_PERMISSION")]
    UnauthorizedPermission,
    #[error("CTAP1_ERR_OTHER")]
    Other,
    #[error("unknown CTAP status code 0x{0:02x}")]
    Unknown(u8),
}

impl CtapStatusCode {
    pub const fn from_byte(value: u8) -> Self {
        match value {
            0x00 => Self::Ok,
            0x01 => Self::InvalidCommand,
            0x02 => Self::InvalidParameter,
            0x03 => Self::InvalidLength,
            0x04 => Self::InvalidSequence,
            0x05 => Self::Timeout,
            0x06 => Self::ChannelBusy,
            0x0a => Self::LockRequired,
            0x0b => Self::InvalidChannel,
            0x11 => Self::CborUnexpectedType,
            0x12 => Self::InvalidCbor,
            0x14 => Self::MissingParameter,
            0x15 => Self::LimitExceeded,
            0x16 => Self::UnsupportedExtension,
            0x17 => Self::FingerprintDatabaseFull,
            0x18 => Self::LargeBlobStorageFull,
            0x19 => Self::CredentialExcluded,
            0x21 => Self::Processing,
            0x22 => Self::InvalidCredential,
            0x23 => Self::UserActionPending,
            0x24 => Self::OperationPending,
            0x25 => Self::NoOperations,
            0x26 => Self::UnsupportedAlgorithm,
            0x27 => Self::OperationDenied,
            0x28 => Self::KeyStoreFull,
            0x2a => Self::NoOperationPending,
            0x2b => Self::UnsupportedOption,
            0x2c => Self::InvalidOption,
            0x2d => Self::KeepaliveCancel,
            0x2e => Self::NoCredentials,
            0x2f => Self::UserActionTimeout,
            0x30 => Self::NotAllowed,
            0x31 => Self::PinInvalid,
            0x32 => Self::PinBlocked,
            0x33 => Self::PinAuthInvalid,
            0x34 => Self::PinAuthBlocked,
            0x35 => Self::PinNotSet,
            0x36 => Self::PinUvAuthTokenRequired,
            0x37 => Self::PinPolicyViolation,
            0x38 => Self::PinTokenExpired,
            0x39 => Self::RequestTooLarge,
            0x3a => Self::ActionTimeout,
            0x3b => Self::UserPresenceRequired,
            0x3c => Self::UserVerificationBlocked,
            0x3d => Self::IntegrityFailure,
            0x3e => Self::InvalidSubcommand,
            0x3f => Self::UserVerificationInvalid,
            0x40 => Self::UnauthorizedPermission,
            0x7f => Self::Other,
            value => Self::Unknown(value),
        }
    }

    pub const fn as_byte(self) -> u8 {
        match self {
            Self::Ok => 0x00,
            Self::InvalidCommand => 0x01,
            Self::InvalidParameter => 0x02,
            Self::InvalidLength => 0x03,
            Self::InvalidSequence => 0x04,
            Self::Timeout => 0x05,
            Self::ChannelBusy => 0x06,
            Self::LockRequired => 0x0a,
            Self::InvalidChannel => 0x0b,
            Self::CborUnexpectedType => 0x11,
            Self::InvalidCbor => 0x12,
            Self::MissingParameter => 0x14,
            Self::LimitExceeded => 0x15,
            Self::UnsupportedExtension => 0x16,
            Self::FingerprintDatabaseFull => 0x17,
            Self::LargeBlobStorageFull => 0x18,
            Self::CredentialExcluded => 0x19,
            Self::Processing => 0x21,
            Self::InvalidCredential => 0x22,
            Self::UserActionPending => 0x23,
            Self::OperationPending => 0x24,
            Self::NoOperations => 0x25,
            Self::UnsupportedAlgorithm => 0x26,
            Self::OperationDenied => 0x27,
            Self::KeyStoreFull => 0x28,
            Self::NoOperationPending => 0x2a,
            Self::UnsupportedOption => 0x2b,
            Self::InvalidOption => 0x2c,
            Self::KeepaliveCancel => 0x2d,
            Self::NoCredentials => 0x2e,
            Self::UserActionTimeout => 0x2f,
            Self::NotAllowed => 0x30,
            Self::PinInvalid => 0x31,
            Self::PinBlocked => 0x32,
            Self::PinAuthInvalid => 0x33,
            Self::PinAuthBlocked => 0x34,
            Self::PinNotSet => 0x35,
            Self::PinUvAuthTokenRequired => 0x36,
            Self::PinPolicyViolation => 0x37,
            Self::PinTokenExpired => 0x38,
            Self::RequestTooLarge => 0x39,
            Self::ActionTimeout => 0x3a,
            Self::UserPresenceRequired => 0x3b,
            Self::UserVerificationBlocked => 0x3c,
            Self::IntegrityFailure => 0x3d,
            Self::InvalidSubcommand => 0x3e,
            Self::UserVerificationInvalid => 0x3f,
            Self::UnauthorizedPermission => 0x40,
            Self::Other => 0x7f,
            Self::Unknown(value) => value,
        }
    }

    pub const fn is_success(self) -> bool {
        matches!(self, Self::Ok)
    }
}
