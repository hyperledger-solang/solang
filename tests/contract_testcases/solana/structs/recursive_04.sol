abstract contract Recursive {
    struct B { B[] b; }
    struct C { B[] b; C c; }
}
