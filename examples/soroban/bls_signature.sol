// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/bls_signature
pragma solidity ^0.8.20;

/// A custom account contract that checks a BLS signature aggregated from
/// several signers (`FastAggregateVerify`). Demonstration only: not audited.
contract bls_signature {
    error InvalidSignature();

    bytes persistent owners;
    bytes instance dst;
    uint32 instance counter;

    /// `agg_pk` is the sum of all signers' G1 public keys. All signers sign
    /// the same message, so one aggregated key is enough.
    constructor(bytes agg_pk) {
        owners = agg_pk;
        dst = "BLSSIG-V01-CS01-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";
        counter = 0;
    }

    /// The business logic this account guards.
    function increment() public returns (uint32) {
        address(this).requireAuth();
        counter += 1;
        return counter;
    }

    /// Checks `e(agg_pk, H(payload)) == e(g1, agg_sig)` as
    /// `e(agg_pk, H(payload)) * e(-g1, agg_sig) == 1`.
    function __check_auth(
        bytes32 signature_payload,
        bytes agg_sig,
        bytes[] auth_contexts
    ) public view {
        // The negated generator point of the G1 group.
        bytes neg_g1 = hex"17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb114d1d6855d545a8aa7d76c8cf2e21f267816aef1db507c96655b9d5caac42364e6f38ba0ecb751bad54dcd6b939c2ca";
        bytes msg_g2 = bls12_381_hash_to_g2(bytes(signature_payload), dst);

        bytes[] memory vp1 = new bytes[](2);
        vp1[0] = owners;
        vp1[1] = neg_g1;

        bytes[] memory vp2 = new bytes[](2);
        vp2[0] = msg_g2;
        vp2[1] = agg_sig;

        if (!bls12_381_pairing_check(vp1, vp2)) {
            revert InvalidSignature();
        }
    }
}
