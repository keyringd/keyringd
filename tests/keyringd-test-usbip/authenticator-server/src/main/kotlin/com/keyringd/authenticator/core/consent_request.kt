package com.keyringd.authenticator.core

import kotlinx.coroutines.CompletableDeferred

internal data class ConsentRequest(
    val id: Long,
    val kind: ConsentKind,
    val rpId: String?,
    val userPresenceRequired: Boolean,
    val userVerificationRequired: Boolean,
    val permissions: Set<String>,
    val response: CompletableDeferred<Boolean>,
)
