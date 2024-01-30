import "polkadot";

contract CallerIsRoot {
    uint64 public balance = 0;

    function admin() public {
        if (caller_is_root()) {
            balance = 0xdeadbeef;
        }
    }
}
