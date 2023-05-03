abstract contract Recursive {
    struct B { B[] b; }
    struct C { B[] b; C c; }
}

// ---- Expect: diagnostics ----
// error: 3:12-13: struct 'C' has infinite size
// 	note 3:23-26: recursive field 'c'
