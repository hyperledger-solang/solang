// SPDX-License-Identifier: BUSL-1.1

pragma solidity ^0.8.0;

import "lib/Lib.sol";
// NOTE: This should be imported from the root of this example; that is,
// this import should be resolved to `02_solang_incorrect_direct_imports/Ambiguous.sol`
import "Ambiguous.sol";

contract Contract {
    using Lib for Lib.Item;

    Lib.Item internal x;

    function addSigner(uint256 account) external {
        x.add(account);
    }
}
