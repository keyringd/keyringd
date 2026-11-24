package com.keyringd.authenticator.app

import com.keyringd.authenticator.app.controller.ControlServer
import com.keyringd.authenticator.core.AuthenticatorRegistry
import com.keyringd.authenticator.core.transport.hybrid.HybridTunnelServer
import com.webauthn4j.ctap.authenticator.transport.usbip.USBIPServer
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.runBlocking

fun main(arguments: Array<String>) {
    val options = AuthenticatorServerOptions.parse(arguments)
    require((options.hybridCertificateChain == null) == (options.hybridPrivateKey == null)) {
        "--hybrid-certificate-chain and --hybrid-private-key must be specified together"
    }
    val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    val usb = USBIPServer(options.usbHost, options.usbPort)
    val registry = AuthenticatorRegistry(usb)
    runBlocking {
        repeat(options.initialAuthenticators) { registry.create(manualApproval = options.manualApproval) }
        usb.start(scope)
    }
    val control = ControlServer(options.controlHost, options.controlPort, registry)
    val tunnel = options.hybridCertificateChain?.let { chain ->
        HybridTunnelServer(
            options.hybridHost, options.hybridPort, chain, requireNotNull(options.hybridPrivateKey),
        ) { routingId, tunnelId ->
            for (key in registry.all()) key.hybridSessions.serve(this, routingId, tunnelId)
        }
    }
    Runtime.getRuntime().addShutdownHook(Thread {
        control.close()
        tunnel?.close()
        runBlocking { usb.stop() }
        scope.cancel()
    })
    control.start()
    tunnel?.start()
    println("USB/IP authenticator server listening on ${options.usbHost}:${options.usbPort}")
    println("Authenticator control API listening on ${options.controlHost}:${options.controlPort}")
    if (tunnel != null) println("Hybrid tunnel listening on ${options.hybridHost}:${options.hybridPort}")
    Thread.currentThread().join()
}
