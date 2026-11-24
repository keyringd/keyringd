package com.keyringd.authenticator.core

internal enum class ConsentKind(val wireName: String) {
    MAKE_CREDENTIAL("make-credential"),
    GET_ASSERTION("get-assertion"),
    PIN_UV_AUTH_TOKEN("pin-uv-auth-token"),
    SELECTION("selection"),
}
