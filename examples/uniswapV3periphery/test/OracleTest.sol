// SPDX-License-Identifier: UNLICENSED
pragma solidity =0.7.6;

import '../libraries/OracleLibrary.sol';

contract OracleTest {
    function consult(address pool, uint32 period) public view returns (int24 timeWeightedAverageTick) {
        timeWeightedAverageTick = OracleLibrary.consult(pool, period);
    }

    function getQuoteAtTick(
        int24 tick,
        uint128 baseAmount,
        address baseToken,
        address quoteToken
    ) public pure returns (uint256 quoteAmount) {
        quoteAmount = OracleLibrary.getQuoteAtTick(tick, baseAmount, baseToken, quoteToken);
    }

    // For gas snapshot test
    function getGasCostOfConsult(address pool, uint32 period) public view returns (uint256) {
        uint256 gasBefore = gasleft();
        OracleLibrary.consult(pool, period);
        return gasBefore - gasleft();
    }

    function getGasCostOfGetQuoteAtTick(
        int24 tick,
        uint128 baseAmount,
        address baseToken,
        address quoteToken
    ) public view returns (uint256) {
        uint256 gasBefore = gasleft();
        OracleLibrary.getQuoteAtTick(tick, baseAmount, baseToken, quoteToken);
        return gasBefore - gasleft();
    }
}
