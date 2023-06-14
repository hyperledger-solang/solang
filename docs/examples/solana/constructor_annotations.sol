@program_id("Foo5mMfYo5RhRcWa4NZ2bwFn4Kdhe8rNK5jchxsKrivA")
contract Foo {

    @space(500 + 12)
    @seed("Foo")
    @payer(payer)
    constructor(@seed bytes seed_val, @bump bytes1 bump_val) {
        // ...
    }
}
