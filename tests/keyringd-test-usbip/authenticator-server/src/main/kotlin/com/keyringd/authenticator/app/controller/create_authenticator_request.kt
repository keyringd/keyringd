package com.keyringd.authenticator.app.controller

import com.keyringd.authenticator.core.LargeBlobSupport
import kotlinx.serialization.Serializable

@Serializable
internal data class CreateAuthenticatorRequest(
    val profile: String = "uv-configured-pin-unconfigured",
    val manualApproval: Boolean = false,
    val largeBlobSupport: LargeBlobSupport = LargeBlobSupport.LARGE_BLOB,
)
