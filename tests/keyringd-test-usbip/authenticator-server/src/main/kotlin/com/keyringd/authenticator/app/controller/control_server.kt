package com.keyringd.authenticator.app.controller

import com.keyringd.authenticator.core.AuthenticatorRegistry
import com.keyringd.authenticator.core.AuthenticatorProfile
import com.keyringd.authenticator.core.ConsentRequest
import com.keyringd.authenticator.core.VirtualAuthenticator
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.kotlinx.json.json
import io.ktor.server.application.ApplicationCall
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.request.receiveText
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.delete
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import kotlinx.serialization.json.Json

internal class ControlServer(host: String, port: Int, private val registry: AuthenticatorRegistry) : AutoCloseable {
    private val server = embeddedServer(CIO, host = host, port = port) {
        install(ContentNegotiation) { json() }
        routing {
            get("/health") { call.respondText("ok\n") }
            get("/authenticators") { call.respond(registry.all().map(::response)) }
            post("/authenticators") {
                call.control {
                    val body = call.receiveText()
                    val options = if (body.isBlank()) CreateAuthenticatorRequest() else Json.decodeFromString<CreateAuthenticatorRequest>(body)
                    val profile = AuthenticatorProfile.entries.firstOrNull {
                        it.name.lowercase().replace('_', '-') == options.profile
                    } ?: throw IllegalArgumentException("Unknown authenticator profile")
                    call.respond(HttpStatusCode.Created, response(registry.create(profile, options.manualApproval, options.largeBlobSupport)))
                }
            }
            route("/authenticators/{authenticatorId}") {
                get {
                    val key = call.key() ?: return@get
                    call.respond(response(key))
                }
                delete {
                    val key = call.key() ?: return@delete
                    registry.delete(key.id)
                    call.respondText("deleted\n")
                }
                post("/attach") {
                    val key = call.key() ?: return@post
                    registry.attach(key.id)
                    call.respond(response(key))
                }
                post("/detach") {
                    val key = call.key() ?: return@post
                    registry.detach(key.id)
                    call.respond(response(key))
                }
                post("/mode/{mode}") {
                    val key = call.key() ?: return@post
                    call.control {
                        when (call.parameters["mode"]) {
                            "manual" -> key.manager.useManualApproval()
                            "automatic" -> key.manager.useAutomaticApproval()
                            else -> throw IllegalArgumentException("Unknown approval mode")
                        }
                        call.respondText("configured\n")
                    }
                }
                post("/configuration/{uv}/{pin}") {
                    val key = call.key() ?: return@post
                    call.control {
                        val profile = when (call.parameters["uv"] to call.parameters["pin"]) {
                            "uv-configured" to "pin-configured" -> AuthenticatorProfile.UV_CONFIGURED_PIN_CONFIGURED
                            "uv-configured" to "pin-unconfigured" -> AuthenticatorProfile.UV_CONFIGURED_PIN_UNCONFIGURED
                            "uv-unconfigured" to "pin-configured" -> AuthenticatorProfile.UV_UNCONFIGURED_PIN_CONFIGURED
                            "uv-unconfigured" to "pin-unconfigured" -> AuthenticatorProfile.UV_UNCONFIGURED_PIN_UNCONFIGURED
                            else -> throw IllegalArgumentException("Unknown authenticator configuration")
                        }
                        key.manager.configure(profile)
                        call.respondText("configured\n")
                    }
                }
                get("/requests/pending") {
                    val key = call.key() ?: return@get
                    call.respond(key.manager.pendingRequests().map(ConsentRequest::toResponse))
                }
                post("/requests/{id}/{answer}") {
                    val key = call.key() ?: return@post
                    val id = call.parameters["id"]?.toLongOrNull()
                    val approved = when (call.parameters["answer"]) {
                        "approve" -> true
                        "reject" -> false
                        else -> null
                    }
                    if (id != null && approved != null && key.manager.answer(id, approved)) call.respondText("answered\n")
                    else call.respondText("request is not pending\n", status = HttpStatusCode.NotFound)
                }
                post("/hybrid/scan") {
                    val key = call.key() ?: return@post
                    call.control {
                        val result = key.hybridSessions.scan(call.receiveText().trim())
                        call.respond(HybridAdvertisementResponse(result.serviceUuid, result.serviceDataHex, result.tunnelDomain, result.tunnelPath))
                    }
                }
            }
        }
    }

    private fun response(key: VirtualAuthenticator) = AuthenticatorResponse(
        key.id, key.device.config.busId, registry.isConnected(key), key.manager.consentRequestCount(),
    )

    private suspend fun ApplicationCall.key(): VirtualAuthenticator? {
        val key = registry.find(parameters["authenticatorId"].orEmpty())
        if (key == null) respondText("unknown authenticator\n", status = HttpStatusCode.NotFound)
        return key
    }

    fun start() { server.start(wait = false) }
    override fun close() { server.stop(gracePeriodMillis = 0, timeoutMillis = 1_000) }
}

private fun ConsentRequest.toResponse() = PendingConsentResponse(id, kind.wireName, rpId, userPresenceRequired, userVerificationRequired, permissions)

private suspend inline fun ApplicationCall.control(action: () -> Unit) {
    try { action() }
    catch (error: IllegalArgumentException) { respondText(error.message.orEmpty() + "\n", status = HttpStatusCode.BadRequest) }
    catch (error: IllegalStateException) { respondText(error.message.orEmpty() + "\n", status = HttpStatusCode.Conflict) }
}
