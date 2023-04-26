function x() immutable {}
contract t {
    function y() immutable public { }
}

// ----
// error (13-22): function cannot be declared 'immutable'
// error (56-65): function cannot be declared 'immutable'
