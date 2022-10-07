import "mangling_02.sol" as X;

struct A { uint256 foo; }

contract C {
       function foo(A memory s) public pure {}
       function foo(X.B memory s) public pure {}
}