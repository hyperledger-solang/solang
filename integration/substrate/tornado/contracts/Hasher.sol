import "substrate";

contract Hasher {
    function MiMCSponge(uint256 xL, uint256 xR) external returns (uint256 outL, uint256 outR) {
        (uint32 ret, bytes output) = chain_extension(220, abi.encode(xL, xR));
        assert(ret == 0);
        (outL, outR) = abi.decode(output, (uint256, uint256));
    }
}
