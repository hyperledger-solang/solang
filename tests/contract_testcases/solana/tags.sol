contract C {
    /**
    * @return c bla
    * @return d bla
    */
    function f1() pure public returns (int c, int d) { return (1, 2); }

    /**
    * @return c bla
    * @return d bla
    */
    function f2() pure public returns (int c, int) { return (1, 2); }

    /**
    * @return feh
    * @return foo
    */
    function f3() pure public returns (int, int) { return (1, 2); }

    /**
    * @return feh
    * @return foo
    */
    function f4() pure public returns (int a, int b) { return (1, 2); }

    /**
    * @return feh
    * @return foo
    */
    function f5() pure public returns (int, int b) { return (1, 2); }
}

// ---- Expect: diagnostics ----
// error: 21:15-18: function return value named 'feh' not found
// error: 22:15-18: function return value named 'foo' not found
// error: 28:15-18: function return value named 'foo' not found
