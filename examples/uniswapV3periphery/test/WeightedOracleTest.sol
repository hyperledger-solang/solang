// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.7.6;
pragma abicoder v2;

import '../libraries/WeightedOracleLibrary.sol';

contract WeightedOracleTest {
    function consult(address pool, uint32 period)
        public
        view
        returns (WeightedOracleLibrary.PeriodObservation memory observation)
    {
        observation = WeightedOracleLibrary.consult(pool, period);
    }

    function getArithmeticMeanTickWeightedByLiquidity(WeightedOracleLibrary.PeriodObservation[] memory observations)
        public
        pure
        returns (int24 arithmeticMeanWeightedTick)
    {
        arithmeticMeanWeightedTick = WeightedOracleLibrary.getArithmeticMeanTickWeightedByLiquidity(observations);
    }
}
