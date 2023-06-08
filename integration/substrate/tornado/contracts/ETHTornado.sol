// https://tornado.cash
/*
 * d888888P                                           dP              a88888b.                   dP
 *    88                                              88             d8'   `88                   88
 *    88    .d8888b. 88d888b. 88d888b. .d8888b. .d888b88 .d8888b.    88        .d8888b. .d8888b. 88d888b.
 *    88    88'  `88 88'  `88 88'  `88 88'  `88 88'  `88 88'  `88    88        88'  `88 Y8ooooo. 88'  `88
 *    88    88.  .88 88       88    88 88.  .88 88.  .88 88.  .88 dP Y8.   .88 88.  .88       88 88    88
 *    dP    `88888P' dP       dP    dP `88888P8 `88888P8 `88888P' 88  Y88888P' `88888P8 `88888P' dP    dP
 * ooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo
 */

// SPDX-License-Identifier: MIT
pragma solidity ^0.7.0;

import "./Tornado.sol";

contract ETHTornado is Tornado {
    constructor(
        IVerifier _verifier,
        IHasher _hasher,
        uint128 _denomination,
        uint32 _merkleTreeHeight
    ) Tornado(_verifier, _hasher, _denomination, _merkleTreeHeight) {}

    function _processDeposit() internal view override {
        require(msg.value == denomination, "Please send `mixDenomination` ETH along with transaction");
    }

    function _processWithdraw(
        address payable _recipient,
        address payable _relayer,
        uint128 _fee,
        uint128 _refund
    ) internal override {
        // sanity checks
        require(msg.value == 0, "Message value is supposed to be zero for ETH instance");
        require(_refund == 0, "Refund value is supposed to be zero for ETH instance");
        require(_fee == 0, "Relayers are not used with this PoC");
        require(uint256(_relayer) == 0, "Relayers are not used with this PoC");
        print("{:x}".format(_recipient));
        _recipient.transfer(denomination);
    }
}
