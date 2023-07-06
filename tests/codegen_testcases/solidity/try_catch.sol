// RUN: --target polkadot --emit cfg
interface I {
	function bar() external returns (int32, bool);
}

contract C {
// BEGIN-CHECK: C::C::function::foo
	function foo(I i) public returns (int32 x) {
        try i.bar() {
            // ensure no abi decoding is done because the return values of bar() are not used
            x = 1;
        } catch (bytes) {
            x = 2;
        }
// NOT-CHECK: builtin ReadFromBuffer ((external call return data)
	}
}
