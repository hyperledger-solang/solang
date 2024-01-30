import "polkadot";

contract CallerIsRoot {
    function is_root() public view returns (bool) {
        return caller_is_root();
    }
}
