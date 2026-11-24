package com.keyringd.authenticator.core

internal enum class AuthenticatorProfile(
    val userVerificationConfigured: Boolean,
    val clientPinConfigured: Boolean,
) {
    UV_CONFIGURED_PIN_CONFIGURED(true, true),
    UV_CONFIGURED_PIN_UNCONFIGURED(true, false),
    UV_UNCONFIGURED_PIN_CONFIGURED(false, true),
    UV_UNCONFIGURED_PIN_UNCONFIGURED(false, false),
}
