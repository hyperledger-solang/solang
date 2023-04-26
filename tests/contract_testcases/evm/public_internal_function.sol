contract foo {
    struct A {
        function() internal a;
    }

    A[] public map;
}

// ----
// error (72-75): variable of type internal function cannot be 'public'
