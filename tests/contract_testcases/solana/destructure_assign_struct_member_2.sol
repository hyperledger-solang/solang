// https://github.com/hyperledger-solang/solang/issues/731
pragma solidity 0.6.12;

interface IUniswapV2Pair {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
}


contract Contract {

    struct Struct1 {
        uint256 a;
        uint256 b;
    }

    function test(address[] memory _tokens) external view {
	uint size = 3;

        // get shares and eth required for each share
        Struct1[] memory struct_1 = new Struct1[](size);

        (struct_1[0].a, struct_1[0].b,) = IUniswapV2Pair.getReserves{program_id: _tokens[0]}();

    }
}

// ---- Expect: diagnostics ----
// warning: 20:26-34: local variable 'struct_1' has been assigned, but never read
// warning: 20:51-55: conversion truncates uint256 to uint32, as memory size is type uint32 on target Solana
