import "polkadot";

contract c {
    bytes32 current;

    function set(Hash h) public returns (Hash) {
        current = Hash.unwrap(h);
        return Hash.wrap(current);
    }
}
