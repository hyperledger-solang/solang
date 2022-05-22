import "simple.sol" as IMP;

int constant override no = 1;

contract C is IMP.A {
	int public override(IMP.A) meh;
}

interface A {
    function foo() external returns (uint);
}
interface B {
    function foo() external returns (uint);
}
contract X is A, B {
        uint public override(A, B) foo;
}
contract Y is X {
}

abstract contract A2 {
    function foo() virtual external returns (uint) { return 1; }
}
abstract contract B2 {
    function foo() virtual external returns (uint) { return 2; }
}
contract X2 is A2, B2 {
        uint public override(A2) foo;
}
contract Y2 is X2 {
}

