
contract BeeeefRegistry {

    struct Entry {
        uint index;
        uint64 timestamp;
        address account;
        address token;
    }

    struct Data {
        bool initialised;
        mapping(bytes32 => Entry) entries;
        bytes32[] index;
    }

    Data private entries;


    function getEntries() public view returns (address[] memory accounts) {
        uint length = 23;
        accounts = new address[](length);
        for (uint i = 0; i < length; i++) {
            Entry memory entry = entries.entries[entries.index[i]];
            accounts[i] = entry.account;
        }
    }
}
// ---- Expect: diagnostics ----
// warning: 22:34-40: conversion truncates uint256 to uint32, as memory size is type uint32 on target Polkadot
