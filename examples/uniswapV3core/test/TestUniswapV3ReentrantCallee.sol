// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.7.6;

import '../libraries/TickMath.sol';

import '../interfaces/callback/IUniswapV3SwapCallback.sol';

import '../UniswapV3Pool.sol';

contract TestUniswapV3ReentrantCallee is IUniswapV3SwapCallback {
    string private constant expectedReason = 'LOK';

    /// @dev The minimum value that can be returned from #getSqrtRatioAtTick. Equivalent to getSqrtRatioAtTick(MIN_TICK)
    uint160 internal constant MIN_SQRT_RATIO = 4295128739;
    /// @dev The maximum value that can be returned from #getSqrtRatioAtTick. Equivalent to getSqrtRatioAtTick(MAX_TICK)
    uint160 internal constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;

    function swapToReenter(address pool) external {
        (,) = UniswapV3Pool(pool).swap(address(0), false, 1, MAX_SQRT_RATIO - 1, new bytes(0));
    }

    function uniswapV3SwapCallback(
        int256,
        int256,
        bytes calldata
    ) external override {
        // try to reenter swap
        try UniswapV3Pool(msg.sender).swap(address(0), false, 1, 0, new bytes(0)) returns (int256 amount0, int256 amount1) {} catch (
            bytes memory reason
        ) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        // try to reenter mint
        try UniswapV3Pool(msg.sender).mint(address(0), 0, 0, 0, new bytes(0)) returns (uint256 amount0, uint256 amount1) {} catch (bytes memory reason) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        // try to reenter collect
        try UniswapV3Pool(msg.sender).collect(address(0), 0, 0, 0, 0) returns (uint128 amount0, uint128 amount1) {} catch (bytes memory reason) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        // try to reenter burn
        try UniswapV3Pool(msg.sender).burn(0, 0, 0) returns (uint256 amount0, uint256 amount1) {} catch (bytes memory reason) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        // try to reenter flash
        try UniswapV3Pool(msg.sender).flash(address(0), 0, 0, new bytes(0)) {} catch (bytes memory reason) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        // try to reenter collectProtocol
        try UniswapV3Pool(msg.sender).collectProtocol(address(0), 0, 0) returns (uint128 amount0, uint128 amount1) {} catch (bytes memory reason) {
            require(keccak256(abi.encode(reason)) == keccak256(abi.encode(expectedReason)));
        }

        require(false, 'Unable to reenter');
    }
}
