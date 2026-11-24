package com.keyringd.authenticator.core.transport.hybrid

import io.ktor.websocket.Frame
import io.ktor.websocket.send
import java.net.ServerSocket
import java.net.URI
import java.net.http.HttpClient
import java.net.http.WebSocket
import java.nio.ByteBuffer
import java.nio.file.Files
import java.nio.file.Path
import java.security.cert.CertificateFactory
import java.security.KeyStore
import java.time.Duration
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManagerFactory
import kotlinx.coroutines.channels.consumeEach
import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

internal class HybridTunnelServerTest {
    @Test
    fun `TLS listener negotiates fido cable and delivers binary frames`() {
        // Public test-only identity, generated with rcgen; valid from 2020 to 2099.
        val certificatePath = Path.of(requireNotNull(javaClass.getResource("/hybrid/certificate.pem")).toURI())
        val privateKeyPath = Path.of(requireNotNull(javaClass.getResource("/hybrid/private-key.pem")).toURI())
        val keyStore = KeyStore.getInstance("PKCS12").apply {
            load(null, null)
            val certificate = Files.newInputStream(certificatePath).use {
                CertificateFactory.getInstance("X.509").generateCertificate(it)
            }
            setCertificateEntry("trusted", certificate)
        }
        val trust = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm()).apply {
            init(keyStore)
        }
        val ssl = SSLContext.getInstance("TLS").apply { init(null, trust.trustManagers, null) }
        val port = ServerSocket(0).use { it.localPort }
        val payload = byteArrayOf(1, 2, 3)
        val received = CompletableFuture<ByteArray>()
        val server = HybridTunnelServer(
            "127.0.0.1", port, certificatePath, privateKeyPath,
        ) { routingId, tunnelId ->
            assertEquals("aabbcc", routingId)
            assertEquals("00112233445566778899aabbccddeeff", tunnelId)
            incoming.consumeEach { frame ->
                if (frame is Frame.Binary) {
                    send(Frame.Binary(true, frame.data))
                }
            }
        }
        try {
            server.start()
            val client = HttpClient.newBuilder().sslContext(ssl).connectTimeout(Duration.ofSeconds(5)).build()
            val webSocket = client.newWebSocketBuilder()
                .subprotocols("fido.cable")
                .buildAsync(
                    URI.create("wss://127.0.0.1:$port/cable/connect/aabbcc/00112233445566778899aabbccddeeff"),
                    object : WebSocket.Listener {
                        override fun onBinary(webSocket: WebSocket, data: ByteBuffer, last: Boolean):
                            java.util.concurrent.CompletionStage<*> {
                            val bytes = ByteArray(data.remaining())
                            data.get(bytes)
                            received.complete(bytes)
                            return CompletableFuture.completedFuture(null)
                        }
                    },
                ).get(10, TimeUnit.SECONDS)
            assertEquals("fido.cable", webSocket.subprotocol)
            webSocket.sendBinary(ByteBuffer.wrap(payload), true).get(5, TimeUnit.SECONDS)
            assertArrayEquals(payload, received.get(5, TimeUnit.SECONDS))
            webSocket.abort()
        } finally {
            server.close()
        }
    }
}
