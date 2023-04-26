import "mangling_02.sol" as X;

struct A { uint256 foo; }

contract C {
       function foo(A memory s) public pure {}
       function foo(X.B memory s) public pure {}
}
// ----
// warning (101-102): function parameter 's' has never been read
// warning (150-151): function parameter 's' has never been read
