package com.keyringd.authenticator.core

import com.webauthn4j.ctap.authenticator.CtapAuthenticator
import com.webauthn4j.ctap.authenticator.attestation.PackedBasicAttestationStatementProvider
import com.webauthn4j.ctap.authenticator.data.settings.CredentialSelectorSetting
import com.webauthn4j.ctap.authenticator.data.settings.AlwaysUvSetting
import com.webauthn4j.ctap.authenticator.extension.CredProtectExtensionProcessor
import com.webauthn4j.ctap.authenticator.extension.HMACSecretExtensionProcessor
import com.webauthn4j.ctap.authenticator.extension.LargeBlobKeyExtensionProcessor
import com.webauthn4j.ctap.authenticator.extension.LargeBlobExtensionProcessor
import com.webauthn4j.ctap.authenticator.store.InMemoryAuthenticatorPropertyStore
import com.webauthn4j.ctap.authenticator.transport.usbip.USBIPDevice
import com.webauthn4j.ctap.authenticator.transport.usbip.USBIPDeviceConfig
import com.webauthn4j.data.AuthenticatorTransport
import com.keyringd.authenticator.core.transport.hybrid.HybridSessionManager
import java.security.MessageDigest

internal class VirtualAuthenticator(val id: String, largeBlobSupport: LargeBlobSupport = LargeBlobSupport.LARGE_BLOB) {
    private val store = InMemoryAuthenticatorPropertyStore()
    private val phoneStore = InMemoryAuthenticatorPropertyStore()
    val manager = AuthenticatorManager { profile ->
        store.clear()
        phoneStore.clear()
        if (profile.clientPinConfigured) {
            val pinHash = MessageDigest.getInstance("SHA-256").digest("1234".toByteArray()).copyOf(16)
            store.saveClientPIN(pinHash)
            phoneStore.saveClientPIN(pinHash)
        }
    }
    private val consent = ControlledUserConsentHandler(manager)
    private val authenticator = CtapAuthenticator(
        attestationStatementProvider = PackedBasicAttestationStatementProvider.createWithDemoAttestationKey(),
        userVerificationCapabilityProvider = consent,
        pinUvAuthTokenConsentHandler = consent,
        builtInUserVerificationHandler = consent,
        makeCredentialConsentHandler = consent,
        getAssertionConsentHandler = consent,
        selectionHandler = consent,
        transports = setOf(AuthenticatorTransport.USB),
        extensionProcessors = listOf(CredProtectExtensionProcessor(), HMACSecretExtensionProcessor()) +
            when (largeBlobSupport) {
                LargeBlobSupport.LARGE_BLOB -> listOf(LargeBlobExtensionProcessor())
                LargeBlobSupport.LARGE_BLOB_KEY -> listOf(LargeBlobKeyExtensionProcessor())
                LargeBlobSupport.UNSUPPORTED -> emptyList()
            },
        credentialSelector = CredentialSelectorSetting.CLIENT_PLATFORM,
        authenticatorPropertyStore = store,
        alwaysUv = AlwaysUvSetting.ENABLED,
    )
    init {
    }
    val device = USBIPDevice(
        authenticator,
        USBIPDeviceConfig(busId = "1-$id", busNum = 1, devNum = id.toInt()),
    )
    val hybridSessions = HybridSessionManager(authenticator.copy(
        transports = setOf(AuthenticatorTransport.HYBRID),
        authenticatorPropertyStore = phoneStore,
    ))

}
