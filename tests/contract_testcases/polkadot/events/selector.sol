event E2(bool);

contract A {
    event E2(bytes4);
    event E1(int256);
    event E3(int256) anonymous;

    function a() public returns (bytes32) {
        return E2.selector;
    }

    function b() public returns (bytes32) {
        return E3.selector;
    }
}

function test() returns (bytes32) {
    return A.E2.selector;
}

contract B {
    function a() public returns (bytes32) {
        return E2.selector;
    }

    function b() public returns (bytes32) {
        return A.E2.selector;
    }

    function c() public returns (bytes32) {
        return A.E1.selector;
    }
}

contract C is A {
    function x() public returns (bytes32) {
        return E2.selector;
    }
}

contract D is B {
    function y() public returns (bytes32) {
        return E2.selector;
    }

    function z() public returns (bytes32) {
        return E3.selector;
    }
}

// ---- Expect: diagnostics ----
// error: 9:16-27: multiple definitions of event
// 	note 4:11-13: possible definition of 'E2'
// 	note 1:7-9: possible definition of 'E2'
// error: 13:16-27: anonymous event has no selector
// error: 37:16-27: multiple definitions of event
// 	note 4:11-13: possible definition of 'E2'
// 	note 1:7-9: possible definition of 'E2'
// error: 47:16-18: 'E3' not found
