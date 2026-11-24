package com.keyringd.authenticator.core

import com.webauthn4j.ctap.authenticator.transport.usbip.USBIPServer
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/** Keeps authenticator state independently from whether its USB device is plugged in. */
internal class AuthenticatorRegistry(private val usb: USBIPServer) {
    private val authenticators = ConcurrentHashMap<String, VirtualAuthenticator>()
    private val changes = Mutex()
    private var nextId = 1

    fun find(id: String): VirtualAuthenticator? = authenticators[id]
    fun all(): List<VirtualAuthenticator> = authenticators.values.sortedBy { it.id.toInt() }
    fun isConnected(key: VirtualAuthenticator): Boolean = usb.devices().contains(key.device)

    suspend fun create(
        profile: AuthenticatorProfile = AuthenticatorProfile.UV_CONFIGURED_PIN_UNCONFIGURED,
        manualApproval: Boolean = false,
        largeBlobSupport: LargeBlobSupport = LargeBlobSupport.LARGE_BLOB,
    ): VirtualAuthenticator = changes.withLock {
        check(nextId <= 65535) { "USB device IDs exhausted" }
        val key = VirtualAuthenticator((nextId++).toString(), largeBlobSupport)
        key.manager.configure(profile)
        if (manualApproval) key.manager.useManualApproval()
        usb.addDevice(key.device)
        authenticators[key.id] = key
        key
    }

    suspend fun attach(id: String): Boolean = changes.withLock {
        val key = authenticators[id] ?: return@withLock false
        if (!isConnected(key)) usb.addDevice(key.device)
        true
    }

    suspend fun detach(id: String): Boolean = changes.withLock {
        val key = authenticators[id] ?: return@withLock false
        usb.removeDevice(key.device.config.busId)
        true
    }

    suspend fun delete(id: String): Boolean = changes.withLock {
        val key = authenticators.remove(id) ?: return@withLock false
        usb.removeDevice(key.device.config.busId)
        true
    }
}
