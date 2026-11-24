package com.keyringd.authenticator.app.controller

import kotlinx.serialization.Serializable

@Serializable
internal data class AuthenticatorResponse(val id: String, val busId: String, val connected: Boolean, val consentRequests: Long)
