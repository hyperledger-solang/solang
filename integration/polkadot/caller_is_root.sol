import "polkadot";

contract CallerIsRoot {
    uint public balance;

    function covert() public payable {
        if (caller_is_root()) {
            balance = 0xdeadbeef;
        } else {
            print("burn more gas");
            balance = 1;
        }
    }
}
