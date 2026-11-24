package com.keyringd.authenticator.core

import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.delay
import org.slf4j.LoggerFactory

internal class AuthenticatorManager(
    private val resetAuthenticator: (AuthenticatorProfile) -> Unit,
) {
    private val logger = LoggerFactory.getLogger(AuthenticatorManager::class.java)
    private val nextId = AtomicLong(1)
    private val consentRequests = AtomicLong(0)
    private val requests = ConcurrentHashMap<Long, ConsentRequest>()

    @Volatile
    private var manual = false

    @Volatile
    private var profile = AuthenticatorProfile.UV_CONFIGURED_PIN_UNCONFIGURED

    @Synchronized
    fun configure(profile: AuthenticatorProfile) {
        check(requests.isEmpty()) { "cannot configure the authenticator while requests are pending" }
        manual = false
        this.profile = profile
        resetAuthenticator(profile)
        logger.info("Configured authenticator profile {}; approval mode reset to automatic", profile)
    }

    fun isUserVerificationConfigured(): Boolean = profile.userVerificationConfigured

    fun useManualApproval() {
        check(requests.isEmpty()) { "cannot change mode while requests are pending" }
        manual = true
        logger.info("Authenticator approval mode changed to manual")
    }

    fun useAutomaticApproval() {
        check(requests.isEmpty()) { "cannot change mode while requests are pending" }
        manual = false
        logger.info("Authenticator approval mode changed to automatic")
    }

    suspend fun requestConsent(
        kind: ConsentKind,
        rpId: String? = null,
        userPresenceRequired: Boolean = true,
        userVerificationRequired: Boolean = false,
        permissions: Set<String> = emptySet(),
    ): Boolean {
        consentRequests.incrementAndGet()
        if (!manual) {
            logger.info("Automatically approving {} consent request", kind)
            delay(AUTOMATIC_APPROVAL_DELAY_MS)
            return true
        }

        val request = ConsentRequest(
            id = nextId.getAndIncrement(),
            kind = kind,
            rpId = rpId,
            userPresenceRequired = userPresenceRequired,
            userVerificationRequired = userVerificationRequired,
            permissions = permissions,
            response = CompletableDeferred(),
        )
        requests[request.id] = request
        logger.info(
            "Queued consent request id={} kind={} rpId={} permissions={}",
            request.id,
            request.kind,
            request.rpId,
            request.permissions,
        )
        return try {
            request.response.await()
        } finally {
            requests.remove(request.id)
            logger.info("Removed consent request id={} kind={}", request.id, request.kind)
        }
    }

    fun pendingRequests(): List<ConsentRequest> = requests.values.sortedBy { it.id }

    fun consentRequestCount(): Long = consentRequests.get()

    fun answer(id: Long, approved: Boolean): Boolean {
        val request = requests.remove(id)
        val answered = request?.response?.complete(approved) == true
        logger.info(
            "Answered consent request id={} kind={} approved={} accepted={}",
            id,
            request?.kind,
            approved,
            answered,
        )
        return answered
    }

    private companion object {
        const val AUTOMATIC_APPROVAL_DELAY_MS = 1_000L
    }
}
