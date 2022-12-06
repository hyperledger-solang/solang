@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {

    @space(500 + 12)
    @seed("Foo")
    @seed(seed_val)
    @bump(bump_val)
    @payer(payer)
    constructor(address payer, bytes seed_val, bytes1 bump_val) {
        // ...
    }
}
