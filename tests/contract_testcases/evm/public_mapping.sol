contract foo {
    struct A {
        mapping(uint64 => uint64) a;
    }

    A[] public map;
}
// ----
// error (78-92): mapping in a struct variable cannot be public
