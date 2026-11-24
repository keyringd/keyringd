package com.keyringd.authenticator.core.transport.hybrid

import com.webauthn4j.ctap.authenticator.CtapAuthenticator
import com.webauthn4j.ctap.authenticator.transport.hybrid.HybridCtapProcessor
import com.webauthn4j.ctap.authenticator.transport.hybrid.HybridDataChannel
import com.webauthn4j.ctap.authenticator.transport.hybrid.HybridQrCtapSession
import com.webauthn4j.ctap.authenticator.transport.hybrid.HybridQrPayload
import com.webauthn4j.ctap.authenticator.transport.hybrid.HybridQrTransaction
import io.ktor.server.websocket.DefaultWebSocketServerSession
import io.ktor.websocket.Frame
import io.ktor.websocket.send
import java.util.concurrent.ConcurrentHashMap
import org.slf4j.LoggerFactory

/** Manages QR-initiated transactions for the virtual phone authenticator. */
internal class HybridSessionManager(private val authenticator: CtapAuthenticator) {
    private val logger = LoggerFactory.getLogger(HybridSessionManager::class.java)
    private val pending = ConcurrentHashMap<String, HybridQrTransaction>()

    fun scan(qrUri: String): HybridQrScanResult {
        val transaction = HybridQrTransaction(HybridQrPayload.fromQrUri(qrUri))
        check(pending.putIfAbsent(transaction.tunnelPath, transaction) == null) {
            "Hybrid tunnel ID collision"
        }
        logger.info("Scanned Hybrid QR; advertisement prepared for {}", transaction.tunnelPath)
        return HybridQrScanResult(
            serviceUuid = "fff9",
            serviceDataHex = transaction.advertisement.toHex(),
            tunnelDomain = transaction.tunnelDomain,
            tunnelPath = transaction.tunnelPath,
        )
    }

    suspend fun serve(
        session: DefaultWebSocketServerSession,
        routingId: String,
        tunnelId: String,
    ) {
        val transaction = pending.remove("/cable/connect/$routingId/$tunnelId") ?: return
        logger.info("Hybrid WSS session started for {}", transaction.tunnelPath)
        val channel = object : HybridDataChannel {
            override suspend fun receive(): ByteArray? {
                while (true) {
                    val frame = session.incoming.receiveCatching().getOrNull() ?: return null
                    when (frame) {
                        is Frame.Binary -> return frame.data
                        is Frame.Close -> return null
                        else -> error("Hybrid tunnel received a non-binary WebSocket frame")
                    }
                }
            }

            override suspend fun send(message: ByteArray) {
                session.send(Frame.Binary(true, message))
            }
        }
        try {
            HybridQrCtapSession(
                transaction,
                HybridCtapProcessor(authenticator.createSession()),
            ).serve(channel)
            logger.info("Hybrid Noise and CTAP session completed for {}", transaction.tunnelPath)
        } catch (error: Exception) {
            logger.error("Hybrid session failed for {}", transaction.tunnelPath, error)
            throw error
        }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it.toInt() and 255) }
}
