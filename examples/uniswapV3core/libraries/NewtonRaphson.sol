// SPDX-License-Identifier: MIT
pragma solidity >=0.4.0;

import './INewtonRaphson.sol';

contract NewtonRaphson {
    function iter(
        uint256 denominator,
        uint256 inv
    ) external pure returns (uint256 result) {
        return inv * (2 - denominator * inv);
    }
}
