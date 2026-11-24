package com.keyringd.authenticator.app.controller

import kotlinx.serialization.Serializable

@Serializable
internal data class HybridAdvertisementResponse(
    val serviceUuid: String,
    val serviceDataHex: String,
    val tunnelDomain: String,
    val tunnelPath: String,
)
