// RUN: --target polkadot --emit cfg

contract CallFlags {
    function delegate_call(address _address, uint32 _flags) public returns (bytes ret) {
        (bool ok, ret) = _address.delegatecall{flags: _flags}(hex"deadbeef");
        (ok, ret) = address(this).delegatecall(hex"cafebabe");
        // CHECK: block0: # entry
        // CHECK: ty:address %_address = (arg #0)
        // CHECK: ty:uint32 %_flags = (arg #1)
        // CHECK: ty:bytes %ret = (alloc bytes len uint32 0)
        // CHECK: %success.temp.4 = external call::delegate address:(arg #0) payload:(alloc bytes uint32 4 hex"deadbeef") value:uint128 0 gas:uint64 0 accounts: seeds: contract|function:_ flags:(arg #1)
        // CHECK: ty:bytes %ret = (external call return data)
        // CHECK: %success.temp.5 = external call::delegate address:address((load (builtin GetAddress ()))) payload:(alloc bytes uint32 4 hex"cafebabe") value:uint128 0 gas:uint64 0 accounts: seeds: contract|function:_ flags:
        // CHECK: ty:bytes %ret = (external call return data)
    }
}
