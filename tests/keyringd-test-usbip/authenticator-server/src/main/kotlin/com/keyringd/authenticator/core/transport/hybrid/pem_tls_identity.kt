package com.keyringd.authenticator.core.transport.hybrid

import java.nio.file.Files
import java.nio.file.Path
import java.security.KeyFactory
import java.security.KeyStore
import java.security.cert.CertificateFactory
import java.security.spec.PKCS8EncodedKeySpec
import java.util.Base64

/** Loads Rust-generated PEM into a JVM TLS identity without a persistent keystore. */
internal class PemTlsIdentity private constructor(val keyStore: KeyStore) {
    companion object {
        const val ALIAS = "hybrid"
        fun load(certificateChain: Path, privateKey: Path): PemTlsIdentity {
            val chain = Files.newInputStream(certificateChain).use {
                CertificateFactory.getInstance("X.509").generateCertificates(it).toTypedArray()
            }
            require(chain.isNotEmpty()) { "PEM certificate chain is empty" }
            val pem = Files.readString(privateKey).trim()
            val begin = "-----BEGIN PRIVATE KEY-----"
            val end = "-----END PRIVATE KEY-----"
            require(pem.startsWith(begin) && pem.endsWith(end)) { "expected a PKCS#8 PEM private key" }
            val encoded = Base64.getDecoder().decode(
                pem.removePrefix(begin).removeSuffix(end).filterNot(Char::isWhitespace),
            )
            val key = KeyFactory.getInstance("EC").generatePrivate(PKCS8EncodedKeySpec(encoded))
            val store = KeyStore.getInstance("PKCS12").apply {
                load(null, null)
                setKeyEntry(ALIAS, key, CharArray(0), chain)
            }
            return PemTlsIdentity(store)
        }
    }
}
