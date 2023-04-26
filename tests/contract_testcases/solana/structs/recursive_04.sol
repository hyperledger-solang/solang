abstract contract Recursive {
    struct B { B[] b; }
    struct C { B[] b; C c; }
}

// ----
// error (65-66): struct 'C' has infinite size
// 	note (76-79): recursive field 'c'
