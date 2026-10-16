// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/simple_account
pragma solidity ^0.8.20;

/// A minimal account contract owned by a single ed25519 public key.
contract simple_account {
    bytes32 instance owner;

    constructor(bytes32 public_key) {
        owner = public_key;
    }

    /// Called by the Soroban host whenever `requireAuth()` is used on this
    /// contract's address. It only verifies the signature; the authorization
    /// context is not used in this example.
    function __check_auth(
        bytes32 signature_payload,
        bytes signature,
        Context[] auth_contexts
    ) public view {
        ed25519_verify(owner, signature_payload, signature);
    }
}
