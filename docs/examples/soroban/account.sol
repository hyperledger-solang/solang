// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/account
pragma solidity ^0.8.20;

/// A multisig account contract with a per-token spend limit for operations that are not signed
/// by every signer. Demonstrates how `__check_auth` uses the authorization context to enforce
/// custom authorization policies.
contract account {
    struct AccSignature {
        bytes32 public_key;
        bytes signature;
    }

    error NotEnoughSigners();
    error NegativeAmount();
    error BadSignatureOrder();
    error UnknownSigner();

    mapping(bytes32 => bool) instance signers;
    uint32 instance signer_count;
    mapping(address => int128) instance spend_limits;
    mapping(address => bool) instance has_spend_limit;

    constructor(bytes32[] memory signer_keys) {
        for (uint32 i = 0; i < signer_keys.length; i++) {
            signers[signer_keys[i]] = true;
        }
        signer_count = signer_keys.length;
    }

    /// Limits the token amount that operations not signed by every signer may spend. Changing
    /// this contract needs the authorization of this account itself.
    function add_limit(address token, int128 limit) public {
        address(this).requireAuth();
        spend_limits[token] = limit;
        has_spend_limit[token] = true;
    }

    /// Called by the Soroban host whenever `requireAuth()` is used on this contract's address.
    function __check_auth(
        bytes32 signature_payload,
        AccSignature[] memory signatures,
        Context[] memory auth_contexts
    ) public view {
        authenticate(signature_payload, signatures);

        // There are no limitations when every signer has signed.
        if (signatures.length == signer_count) {
            return;
        }

        // The spend left per token in this authorization, so that several transfers of the same
        // token share one limit.
        address[] memory tokens = new address[](auth_contexts.length);
        int128[] memory spend_left = new int128[](auth_contexts.length);
        uint32 token_count = 0;

        for (uint32 i = 0; i < auth_contexts.length; i++) {
            Context memory ctx = auth_contexts[i];

            // Creating contracts and changing this contract need every signer.
            if (ctx.kind != ContextKind.Contract || ctx.contract_address == address(this)) {
                revert NotEnoughSigners();
            }

            // Only token spending functions are limited, and only for tokens with a limit.
            bool burn = ctx.fn_name == "burn";
            if (!(burn || ctx.fn_name == "transfer" || ctx.fn_name == "approve")) {
                continue;
            }
            if (!has_spend_limit[ctx.contract_address]) {
                continue;
            }

            uint32 t = 0;
            while (t < token_count && tokens[t] != ctx.contract_address) {
                t++;
            }
            if (t == token_count) {
                tokens[t] = ctx.contract_address;
                spend_left[t] = spend_limits[ctx.contract_address];
                token_count++;
            }

            // The amount is the second argument of `burn(from, amount)`, and the third of
            // `transfer(from, to, amount)` and `approve(from, spender, amount, ...)`.
            int128 spent;
            if (burn) {
                (, spent) = abi.decode(ctx.args, (address, int128));
            } else {
                (, , spent) = abi.decode(ctx.args, (address, address, int128));
            }
            if (spent < 0) {
                revert NegativeAmount();
            }
            if (spent > spend_left[t]) {
                revert NotEnoughSigners();
            }
            spend_left[t] -= spent;
        }
    }

    /// Checks that the signatures are from known signers, sorted by public key without
    /// duplicates, and valid.
    function authenticate(bytes32 payload, AccSignature[] memory signatures) internal view {
        for (uint32 i = 0; i < signatures.length; i++) {
            if (i > 0 && signatures[i - 1].public_key >= signatures[i].public_key) {
                revert BadSignatureOrder();
            }
            if (!signers[signatures[i].public_key]) {
                revert UnknownSigner();
            }
            ed25519_verify(signatures[i].public_key, payload, signatures[i].signature);
        }
    }
}
