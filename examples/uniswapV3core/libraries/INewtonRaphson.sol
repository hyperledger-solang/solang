// SPDX-License-Identifier: MIT
pragma solidity >=0.4.0;

interface INewtonRaphson {
    function iter(
        uint256 denominator,
        uint256 inv
    ) external pure returns (uint256 result);
}
