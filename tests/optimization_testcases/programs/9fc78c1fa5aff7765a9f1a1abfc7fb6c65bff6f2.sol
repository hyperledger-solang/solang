contract foo {
    function f() public returns (uint) {
        return 2;
    }

    function g() public returns (uint) {
        return false ? 2 : 3;
    }

    function h() public returns (uint) {
        return true ? f() : g();
    }

    function i() public returns (uint) {
        int a = 24;
        return uint(a);
    }

    function j() public returns (uint) {
        return 2 + 3;
    }
}
