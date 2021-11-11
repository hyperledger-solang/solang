// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.7.6;

import '../libraries/TickMath.sol';

contract TickMathEchidnaTest {
    /// @dev The minimum tick that may be passed to #getSqrtRatioAtTick computed from log base 1.0001 of 2**-128
    int24 internal constant MIN_TICK = -887272;
    /// @dev The maximum tick that may be passed to #getSqrtRatioAtTick computed from log base 1.0001 of 2**128
    int24 internal constant MAX_TICK = -MIN_TICK;

    /// @dev The minimum value that can be returned from #getSqrtRatioAtTick. Equivalent to getSqrtRatioAtTick(MIN_TICK)
    uint160 internal constant MIN_SQRT_RATIO = 4295128739;
    /// @dev The maximum value that can be returned from #getSqrtRatioAtTick. Equivalent to getSqrtRatioAtTick(MAX_TICK)
    uint160 internal constant MAX_SQRT_RATIO = 1461446703485210103287273052203988822378723970342;

    // uniqueness and increasing order
    function checkGetSqrtRatioAtTickInvariants(int24 tick) external pure {
        uint160 ratio = TickMath.getSqrtRatioAtTick(tick);
        assert(TickMath.getSqrtRatioAtTick(tick - 1) < ratio && ratio < TickMath.getSqrtRatioAtTick(tick + 1));
        assert(ratio >= MIN_SQRT_RATIO);
        assert(ratio <= MAX_SQRT_RATIO);
    }

    // the ratio is always between the returned tick and the returned tick+1
    function checkGetTickAtSqrtRatioInvariants(uint160 ratio) external pure {
        int24 tick = TickMath.getTickAtSqrtRatio(ratio);
        assert(ratio >= TickMath.getSqrtRatioAtTick(tick) && ratio < TickMath.getSqrtRatioAtTick(tick + 1));
        assert(tick >= MIN_TICK);
        assert(tick < MAX_TICK);
    }
}
