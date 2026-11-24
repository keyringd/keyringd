package com.keyringd.authenticator.app

import java.nio.file.Path

internal data class AuthenticatorServerOptions(
    val usbHost: String = "127.0.0.1",
    val usbPort: Int = 3240,
    val initialAuthenticators: Int = 0,
    val manualApproval: Boolean = false,
    val controlHost: String = "127.0.0.1",
    val controlPort: Int = 3241,
    val hybridHost: String = "127.0.0.1",
    val hybridPort: Int = 8443,
    val hybridCertificateChain: Path? = null,
    val hybridPrivateKey: Path? = null,
) {
    companion object {
        fun parse(arguments: Array<String>): AuthenticatorServerOptions {
            val values = arguments.associate { argument ->
                require(argument.startsWith("--") && '=' in argument) {
                    "arguments must use --name=value: $argument"
                }
                argument.removePrefix("--").split('=', limit = 2).let { it[0] to it[1] }
            }
            return AuthenticatorServerOptions(
                usbHost = values["usb-host"] ?: "127.0.0.1",
                usbPort = values["usb-port"]?.toInt() ?: 3240,
                initialAuthenticators = (values["initial-authenticators"]?.toInt() ?: 0).also { require(it in 0..65535) },
                manualApproval = values["manual-approval"]?.toBooleanStrict() ?: false,
                controlHost = values["control-host"] ?: "127.0.0.1",
                controlPort = values["control-port"]?.toInt() ?: 3241,
                hybridHost = values["hybrid-host"] ?: "127.0.0.1",
                hybridPort = values["hybrid-port"]?.toInt() ?: 8443,
                hybridCertificateChain = values["hybrid-certificate-chain"]?.let(Path::of),
                hybridPrivateKey = values["hybrid-private-key"]?.let(Path::of),
            )
        }
    }
}
