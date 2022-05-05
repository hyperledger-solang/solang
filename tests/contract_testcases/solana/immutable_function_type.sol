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
