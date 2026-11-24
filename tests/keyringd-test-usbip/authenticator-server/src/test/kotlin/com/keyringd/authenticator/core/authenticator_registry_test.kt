package com.keyringd.authenticator.core

import com.webauthn4j.ctap.authenticator.transport.usbip.USBIPServer
import kotlinx.coroutines.runBlocking
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

internal class AuthenticatorRegistryTest {
    @Test
    fun `detached keys retain their authenticator while deletion discards it`() = runBlocking {
        val usb = USBIPServer()
        val registry = AuthenticatorRegistry(usb)
        val first = registry.create()
        val second = registry.create(AuthenticatorProfile.UV_UNCONFIGURED_PIN_CONFIGURED, true)
        assertEquals(listOf("1-1", "1-2"), usb.devices().map { it.config.busId })
        assertFalse(second.manager.isUserVerificationConfigured())
        assertTrue(first.manager.isUserVerificationConfigured())
        assertTrue(registry.detach(first.id))
        assertSame(first, registry.find(first.id))
        assertFalse(registry.isConnected(first))
        assertTrue(registry.isConnected(second))
        assertTrue(registry.attach(first.id))
        assertSame(first.device, usb.devices().last())
        assertTrue(registry.delete(first.id))
        assertNull(registry.find(first.id))
        assertFalse(registry.attach(first.id))
        val replacement = registry.create()
        assertNotEquals(first.id, replacement.id)
        usb.stop()
    }
}
