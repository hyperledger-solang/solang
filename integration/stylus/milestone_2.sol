// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

contract C {
    function test()
        public
        view
        returns (uint64, uint256, address, uint256, uint256, uint256, uint256)
    {
        uint64 block_gasleft = gasleft();
        uint256 block_basefee = block.basefee;
        address block_coinbase = block.coinbase;
        uint256 block_gaslimit = block.gaslimit;
        uint256 block_number = block.number;
        uint256 block_timestamp = block.timestamp;
        uint256 block_chainid = block.chainid;

        return (
            block_gasleft,
            block_basefee,
            block_coinbase,
            block_gaslimit,
            block_number,
            block_timestamp,
            block_chainid
        );
    }
}
