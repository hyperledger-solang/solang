import "./mangling_02.sol" as X;

struct A { uint256 foo; }

contract C {
       function foo(A memory s) public pure {}
       function foo(X.B memory s) public pure {}
}
// ---- Expect: diagnostics ----
// warning: 6:30-31: function parameter 's' is unused
// warning: 7:32-33: function parameter 's' is unused
