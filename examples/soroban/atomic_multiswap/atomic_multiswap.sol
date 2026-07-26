/// SPDX-License-Identifier: Apache-2.0
// Mapping of https://github.com/stellar/soroban-examples/tree/main/atomic_multiswap

contract atomic_multiswap {
    struct SwapSpec {
        address account;
        uint64 amount;
        uint64 min_recv;
    }

    // Swaps token A for token B between multiple parties in a single transaction.
    //
    // For every entry in `a`, find the first not-yet-matched entry in `b` whose amounts are
    // mutually acceptable, and settle that pair through the `atomic_swap` contract via a
    // cross-contract call. A swap that fails is skipped and the `b` entry stays available.
    //
    // (Solidity memory arrays cannot be shrunk like the upstream `Vec::remove`, so a `matched`
    // mask is used to mark `b` entries that have already been consumed.)
    function multi_swap(
        address swap_contract,
        address token_a,
        address token_b,
        SwapSpec[] memory a,
        SwapSpec[] memory b
    ) public {
        bool[] memory matched = new bool[](b.length);

        for (uint64 i = 0; i < a.length; i++) {
            SwapSpec memory acc_a = a[i];

            for (uint64 j = 0; j < b.length; j++) {
                if (matched[j]) {
                    continue;
                }

                SwapSpec memory acc_b = b[j];

                if (acc_a.amount >= acc_b.min_recv && acc_b.amount >= acc_a.min_recv) {
                    bytes payload = abi.encode(
                        "swap",
                        acc_a.account,
                        acc_b.account,
                        token_a,
                        token_b,
                        acc_a.amount,
                        acc_a.min_recv,
                        acc_b.amount,
                        acc_b.min_recv
                    );

                    (bool success, bytes returndata) = swap_contract.call(payload);

                    if (success) {
                        matched[j] = true;
                        break;
                    }
                }
            }
        }
    }
}
