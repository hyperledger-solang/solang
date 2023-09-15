contract A {
    struct Bar {
        uint foo;
        string bar;
    }
    error Foo(Bar);

    function a() public pure {
        revert Foo(Bar({foo: 123, bar: "bar"}));
    }
}

// ---- Expect: diagnostics ----
