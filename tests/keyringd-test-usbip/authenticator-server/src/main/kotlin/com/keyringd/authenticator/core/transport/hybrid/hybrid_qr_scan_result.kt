package com.keyringd.authenticator.core.transport.hybrid

internal data class HybridQrScanResult(
    val serviceUuid: String,
    val serviceDataHex: String,
    val tunnelDomain: String,
    val tunnelPath: String,
)
