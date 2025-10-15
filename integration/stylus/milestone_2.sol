// SPDX-License-Identifier: UNLICENSED
pragma solidity >=0.8.0;

contract T {
    function test() public view returns (uint256) {
        return 1;
    }
}

contract C {
    bytes32 private constant REENTRANCY_GUARD_STORAGE =
        0x9b779b17422d0df92223018b32b4d1fa46e071723d6817e2486d003becc55f00;

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

    function test2() public returns (uint256 a, uint256 b) {
        assembly {
            tstore(REENTRANCY_GUARD_STORAGE, 1)
            sstore(REENTRANCY_GUARD_STORAGE, 134)
            a := sload(REENTRANCY_GUARD_STORAGE)
            b := tload(REENTRANCY_GUARD_STORAGE)
        }
        return (a, b);
    }
}
