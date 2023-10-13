
interface Foo {
    @signer(acc1)
    @account(acc2)
    function Bar() external;
}

contract Derived is Foo {

    bool b;
    @mutableSigner(acc1)
    @mutableAccount(acc2)
    function Bar()  external {
        b = false;
    }
}

// ---- Expect: diagnostics ----
// error: 11:5-25: account 'acc1' must be declared with the same annotation for overriding
// 	note 3:5-18: location of other declaration
// error: 12:5-26: account 'acc2' must be declared with the same annotation for overriding
// 	note 4:5-19: location of other declaration
