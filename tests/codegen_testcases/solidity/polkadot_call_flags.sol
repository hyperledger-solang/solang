// RUN: --target polkadot --emit cfg

contract CallFlags {
    function call_with_flags( address _address, uint32 _flags) public returns (bytes ret) {
        (bool ok, ret) = _address.call{flags: _flags}(hex"deadbeef");
        // CHECK: block0: # entry
        // CHECK: ty:uint32 %_flags = (arg #1)
        // CHECK: %success.temp.4 = external call::regular address:(arg #0) payload:(alloc bytes uint32 4 hex"deadbeef") value:uint128 0 gas:uint64 0 accounts: seeds: contract|function:_ flags:(arg #1)
    }
}
