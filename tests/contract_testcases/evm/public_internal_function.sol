contract foo {
    struct A {
        function() internal a;
    }

    A[] public map;
}

// ---- Expect: diagnostics ----
// error: 6:5-8: variable of type internal function cannot be 'public'
