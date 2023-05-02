contract c {
    function() internal returns (uint) private immutable f = g;
    function g() internal pure returns (uint) { return 2; }
}

contract d {
    function() internal returns (uint) immutable private f = g;
    function g() internal pure returns (uint) { return 2; }
}

contract e {
    function() internal returns (uint) private immutable immutable f = g;
    function g() internal pure returns (uint) { return 2; }
}
contract b {
    function() internal immutable returns (uint) private f = g;
    function g() internal pure returns (uint) { return 2; }
}

contract a {
    function x() public {
	function() internal returns (uint) private immutable f;
    }
    function y() public {
	function() internal private immutable g;
    }
}

// ---- Expect: diagnostics ----
// error: 12:48-57: duplicate 'immutable' attribute
// 	note 12:58-67: previous 'immutable' attribute
// error: 22:37-44: function type cannot have visibility 'private'
// error: 22:45-54: function type cannot be 'immutable'
// error: 25:22-29: function type cannot have visibility 'private'
// error: 25:30-39: function type cannot be 'immutable'
