package com.keyringd.authenticator.core

import com.webauthn4j.ctap.authenticator.AuthenticatorSelectionHandler
import com.webauthn4j.ctap.authenticator.BuiltInUserVerificationAttemptResult
import com.webauthn4j.ctap.authenticator.BuiltInUserVerificationHandler
import com.webauthn4j.ctap.authenticator.GetAssertionConsentHandler
import com.webauthn4j.ctap.authenticator.GetAssertionConsentRequest
import com.webauthn4j.ctap.authenticator.MakeCredentialConsentHandler
import com.webauthn4j.ctap.authenticator.MakeCredentialConsentRequest
import com.webauthn4j.ctap.authenticator.PinUvAuthTokenConsentHandler
import com.webauthn4j.ctap.authenticator.PinUvAuthTokenConsentRequest
import com.webauthn4j.ctap.authenticator.UserVerificationCapabilityProvider
import com.webauthn4j.ctap.core.data.options.UserVerificationOption

internal class ControlledUserConsentHandler(
    private val manager: AuthenticatorManager,
) : UserVerificationCapabilityProvider,
    MakeCredentialConsentHandler,
    GetAssertionConsentHandler,
    AuthenticatorSelectionHandler,
    PinUvAuthTokenConsentHandler,
    BuiltInUserVerificationHandler {

    override fun getUserVerificationOption(rpId: String?): UserVerificationOption? =
        if (manager.isUserVerificationConfigured()) {
            UserVerificationOption.READY
        } else {
            UserVerificationOption.NOT_READY
        }

    override suspend fun onMakeCredentialConsentRequested(
        makeCredentialConsentRequest: MakeCredentialConsentRequest,
    ): Boolean = manager.requestConsent(
        kind = ConsentKind.MAKE_CREDENTIAL,
        rpId = makeCredentialConsentRequest.rp?.id,
        userPresenceRequired = makeCredentialConsentRequest.isUserPresenceRequired,
        userVerificationRequired = makeCredentialConsentRequest.isUserVerificationRequired,
    )

    override suspend fun onGetAssertionConsentRequested(
        getAssertionConsentRequest: GetAssertionConsentRequest,
    ): Boolean = manager.requestConsent(
        kind = ConsentKind.GET_ASSERTION,
        rpId = getAssertionConsentRequest.rpId,
        userPresenceRequired = getAssertionConsentRequest.isUserPresenceRequired,
        userVerificationRequired = getAssertionConsentRequest.isUserVerificationRequired,
    )

    override suspend fun onSelectionRequested(): Boolean =
        manager.requestConsent(ConsentKind.SELECTION)

    override suspend fun onPinUvAuthTokenConsentRequested(
        request: PinUvAuthTokenConsentRequest,
    ): Boolean = manager.requestConsent(
        kind = ConsentKind.PIN_UV_AUTH_TOKEN,
        rpId = request.rpId,
        userVerificationRequired = true,
        permissions = request.permissions.mapTo(linkedSetOf()) { it.name.lowercase() },
    )

    override suspend fun performBuiltInUserVerification(): BuiltInUserVerificationAttemptResult {
        check(manager.isUserVerificationConfigured()) {
            "built-in user verification is not configured for this authenticator profile"
        }
        return BuiltInUserVerificationAttemptResult.Verified(userPresent = true)
    }
}
