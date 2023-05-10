import "substrate";

contract Foo {
    // Call the "rand-extension" example chain extension demonstrated here:
    // https://use.ink/macros-attributes/chain-extension
    //
    // This chain extension is registered under ID 1101.
    // It takes a bytes32 as input seed and returns a pseudo random bytes32.
    function fetch_random(bytes32 _seed) public returns (bytes32) {
        bytes input = abi.encode(_seed);
        bytes output = chain_extension(1101, input);

        bytes32 random = abi.decode(output, (bytes32));
        print("chain extension 1101 output: {}".format(random));

        return random;
    }
}
