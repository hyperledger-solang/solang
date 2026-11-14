// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/multisig_1_of_n_account
pragma solidity ^0.8.20;

/// A 1-of-n multisig account contract: any one of the configured ed25519
/// public keys may sign.
contract multisig_1_of_n_account {
    struct Signature {
        bytes32 public_key;
        bytes signature;
    }

    error UnknownSigner();

    mapping(bytes32 => bool) instance signers;

    constructor(bytes32[] memory signer_keys) {
        for (uint32 i = 0; i < signer_keys.length; i++) {
            signers[signer_keys[i]] = true;
        }
    }

    /// Called by the Soroban host whenever `requireAuth()` is used on this
    /// contract's address.
    function __check_auth(
        bytes32 signature_payload,
        Signature memory signature,
        Context[] auth_contexts
    ) public view {
        if (!signers[signature.public_key]) {
            revert UnknownSigner();
        }
        ed25519_verify(signature.public_key, signature_payload, signature.signature);
    }
}
