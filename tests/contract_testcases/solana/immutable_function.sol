function x() immutable {}
contract t {
    function y() immutable public { }
}

// ---- Expect: diagnostics ----
// error: 1:14-23: function cannot be declared 'immutable'
// error: 3:18-27: function cannot be declared 'immutable'
