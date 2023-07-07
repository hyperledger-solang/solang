import "polkadot";

contract ChainExtension {
    // Implementation on the node:
    // https://github.com/hyperledger/solang-substrate-ci/blob/substrate-integration/runtime/src/chain_ext.rs#L43
    function fetch_random(bytes32 _seed) public returns (bytes32) {
        bytes seed = abi.encode(_seed);
        (uint32 ret, bytes output) = chain_extension(1101, seed);
        assert(ret == 0);
        return abi.decode(output, (bytes32));
    }
}
