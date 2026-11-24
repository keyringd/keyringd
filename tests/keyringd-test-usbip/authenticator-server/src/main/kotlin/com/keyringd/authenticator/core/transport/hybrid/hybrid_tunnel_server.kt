package com.keyringd.authenticator.core.transport.hybrid

import io.ktor.server.application.install
import io.ktor.server.engine.embeddedServer
import io.ktor.server.engine.sslConnector
import io.ktor.server.netty.Netty
import io.ktor.server.routing.routing
import io.ktor.server.websocket.DefaultWebSocketServerSession
import io.ktor.server.websocket.WebSockets
import io.ktor.server.websocket.webSocket
import java.nio.file.Path

/** TLS and WebSocket endpoint owned by the E2E phone authenticator process. */
internal class HybridTunnelServer(
    host: String,
    port: Int,
    certificateChain: Path,
    privateKey: Path,
    onConnect: suspend DefaultWebSocketServerSession.(routingId: String, tunnelId: String) -> Unit,
) : AutoCloseable {
    private val server = embeddedServer(Netty, configure = {
        val identity = PemTlsIdentity.load(certificateChain, privateKey)
        sslConnector(
            keyStore = identity.keyStore,
            keyAlias = PemTlsIdentity.ALIAS,
            keyStorePassword = { CharArray(0) },
            privateKeyPassword = { CharArray(0) },
        ) {
            this.host = host
            this.port = port
        }
    }) {
        install(WebSockets) {
            maxFrameSize = 1024L * 1024L + 64L
        }
        routing {
            webSocket("/cable/connect/{routingId}/{tunnelId}", protocol = "fido.cable") {
                val routingId = call.parameters["routingId"]
                val tunnelId = call.parameters["tunnelId"]
                if (routingId != null && tunnelId != null &&
                    routingId.matches(Regex("[0-9a-f]{6}")) &&
                    tunnelId.matches(Regex("[0-9a-f]{32}"))) {
                    onConnect(routingId, tunnelId)
                }
            }
        }
    }

    fun start() {
        server.start(wait = false)
    }

    override fun close() {
        server.stop(gracePeriodMillis = 0, timeoutMillis = 1_000)
    }
}
