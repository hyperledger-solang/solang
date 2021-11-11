// SPDX-License-Identifier: MIT

pragma solidity 0.6.12;

interface Allocatable {
    function allocateTo(address, uint256) external;
}