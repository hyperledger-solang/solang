
contract Test1 {
    function sum(int32 a, int32 b) public pure returns (int32) {
        return a + b;
    }

    // This is not allowed on Solana, because the discriminator for both functions is exactly the same.
    function sum(int64 a, int64 b) public pure returns (int64) {
        return a + b;
    }
}

contract Test2 {
    constructor() {

    }

    function sub(int32 d) public pure returns (int32) {
        return d-2;
    }

    function multiply(int32 a, int32 b) public pure returns (int32) {
        return a*b;
    }
}

contract Test3 is Test2 {
    int32 state;
    constructor(int32 state_var) {
        state = state_var;
    }

    function multiply(int32 c) public pure returns (int32) {
        return c*state;
    }
}

contract Test4 is Test2 {
    int32 state;
    constructor(int32 state_var) {
        state = state_var;
    }

    function multiply(int32 a, int32 b) public pure returns (int32) {
        return a*state*b;
    }
}

contract Test5 is Test3 {
    constructor(int32 state_var) Test3(state_var) {}

    function sub(int64 e) public pure returns (int64) {
        return e-2;
    }
}

abstract contract Test6 {
    constructor() {}

    function doThis() public virtual returns (int32);
}

contract Test7 is Test6 {
    constructor() {

    }

    function doThis() public override(Test6) returns (int32) {
        return 7;
    }
}

contract Base1
{
    function foo() virtual public {}
}

contract Base2
{
    function foo() virtual public {}
}

contract Inherited is Base1, Base2
{
    // This should be allowed
    function foo() public override(Base1, Base2) {}
}

contract ManglingInvalid {
    function foo_bool() public pure returns (int32) {
        return 2;
    }

    // This should not be allowed
    function foo(bool a) public pure returns (int32) {
        if (a) {
            return 1;
        } else {
            return 3;
        }
    }
}
// ---- Expect: diagnostics ----
// error: 22:5-68: function 'multiply' with this signature already defined
// 	note 44:5-68: previous definition of function 'multiply'
// error: 95:5-53: mangling the symbol of overloaded function 'foo' with signature 'foo(bool)' results in a new symbol 'foo_bool' but this symbol already exists
// 	note 90:5-52: this function declaration conflicts with mangled name
