package com.keyringd.authenticator.app.controller

import kotlinx.serialization.Serializable

@Serializable
internal data class PendingConsentResponse(
    val id: Long,
    val kind: String,
    val rpId: String?,
    val userPresenceRequired: Boolean,
    val userVerificationRequired: Boolean,
    val permissions: Set<String>,
)
