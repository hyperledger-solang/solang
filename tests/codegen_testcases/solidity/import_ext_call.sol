// RUN: --target solana --emit cfg
import '../import_test.sol' as My;

@program_id("6qEm4QUJGFvqKNJGjTrAEiFhbVBY4ashpBjDHEFvEUmW")
contract Foo {
    // BEGIN-CHECK: Foo::Foo::function::get_b
    function get_b(address id) external pure {
        // External calls
        // CHECK: external call::regular address:(arg #0) payload:%abi_encoded.temp.2 value:uint64 0 gas:uint64 0 accounts:[0] [  ] seeds: contract|function:(2, 2) flags:
        My.Dog.barks{program_id: id}("woof");
        // CHECK: external call::regular address:(arg #0) payload:%abi_encoded.temp.4 value:uint64 0 gas:uint64 0 accounts:[0] [  ] seeds: contract|function:(2, 2) flags:
        My.Dog.barks{program_id: id}({what: "meow"});
    }
}

contract Cat is My.Dog {
    // BEGIN-CHECK: Cat::Cat::function::try_cat
    function try_cat() public pure {
        // Internal calls
        My.Dog.barks("woof");
        // CHECK: Cat::Dog::function::barks__string (alloc string uint32 4 "woof")
        My.Dog.barks({what: "meow"});
        // CHECK: Cat::Dog::function::barks__string (alloc string uint32 4 "meow")
    }
}