package com.keyringd.authenticator.core

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
internal enum class LargeBlobSupport {
    @SerialName("largeBlob") LARGE_BLOB,
    @SerialName("largeBlobKey") LARGE_BLOB_KEY,
    @SerialName("unsupported") UNSUPPORTED,
}
