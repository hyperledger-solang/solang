contract foo {
    struct A {
        mapping(uint64 => uint64) a;
    }

    A[] public map;
}
// ---- Expect: diagnostics ----
// error: 6:5-19: mapping in a struct variable cannot be public
