// SPDX-License-Identifier: GPL-2.0-or-later
pragma solidity >=0.5.0 <0.8.0;

import '../uniswapV3core/interfaces/IUniswapV3Pool.sol';

/// @title Weighted Oracle library
/// @notice Provides functions to integrate with different tier oracles of the same V3 pair
library WeightedOracleLibrary {
    /// @notice The result of observating a pool across a certain period
    struct PeriodObservation {
        int24 arithmeticMeanTick;
        uint128 harmonicMeanLiquidity;
    }

    /// @notice Fetches a time-weighted observation for a given Uniswap V3 pool
    /// @param pool Address of the pool that we want to observe
    /// @param period Number of seconds in the past to start calculating the time-weighted observation
    /// @return observation An observation that has been time-weighted from (block.timestamp - period) to block.timestamp
    function consult(address pool, uint32 period) internal view returns (PeriodObservation memory observation) {
        require(period != 0, 'BP');

        uint192 periodX160 = uint192(period) * type(uint160).max;

        uint32[] memory secondsAgos = new uint32[](2);
        secondsAgos[0] = period;
        secondsAgos[1] = 0;

        (int56[] memory tickCumulatives, uint160[] memory secondsPerLiquidityCumulativeX128s) =
            IUniswapV3Pool(pool).observe(secondsAgos);
        int56 tickCumulativesDelta = tickCumulatives[1] - tickCumulatives[0];
        uint160 secondsPerLiquidityCumulativesDelta =
            secondsPerLiquidityCumulativeX128s[1] - secondsPerLiquidityCumulativeX128s[0];

        observation.arithmeticMeanTick = int24(tickCumulativesDelta / period);
        // Always round to negative infinity
        if (tickCumulativesDelta < 0 && (tickCumulativesDelta % period != 0)) observation.arithmeticMeanTick--;

        // We are shifting the liquidity delta to ensure that the result doesn't overflow uint128
        observation.harmonicMeanLiquidity = uint128(periodX160 / (uint192(secondsPerLiquidityCumulativesDelta) << 32));
    }

    /// @notice Given some time-weighted observations, calculates the arithmetic mean tick, weighted by liquidity
    /// @param observations A list of time-weighted observations
    /// @return arithmeticMeanWeightedTick The arithmetic mean tick, weighted by the observations' time-weighted harmonic average liquidity
    /// @dev In most scenarios, each entry of `observations` should share the same `period` and underlying `pool` tokens.
    /// If `period` differs across observations, the result becomes difficult to interpret and is likely biased/manipulable.
    /// If the underlying `pool` tokens differ across observations, extreme care must be taken to ensure that both prices and liquidity values are comparable.
    /// Even if prices are commensurate (e.g. two different USD-stable assets against ETH), liquidity values may not be, as decimals can differ between tokens.
    function getArithmeticMeanTickWeightedByLiquidity(PeriodObservation[] memory observations)
        internal
        pure
        returns (int24 arithmeticMeanWeightedTick)
    {
        // Accumulates the sum of all observations' products between each their own average tick and harmonic average liquidity
        // Each product can be stored in a int160, so it would take approximatelly 2**96 observations to overflow this accumulator
        int256 numerator;

        // Accumulates the sum of the harmonic average liquidities from the given observations
        // Each average liquidity can be stored in a uint128, so it will take approximatelly 2**128 observations to overflow this accumulator
        uint256 denominator;

        for (uint256 i; i < observations.length; i++) {
            numerator += int256(observations[i].harmonicMeanLiquidity) * observations[i].arithmeticMeanTick;
            denominator += observations[i].harmonicMeanLiquidity;
        }

        arithmeticMeanWeightedTick = int24(numerator / int256(denominator));

        // Always round to negative infinity
        if (numerator < 0 && (numerator % int256(denominator) != 0)) arithmeticMeanWeightedTick--;
    }
}
