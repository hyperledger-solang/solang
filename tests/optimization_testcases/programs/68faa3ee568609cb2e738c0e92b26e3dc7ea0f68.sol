library SafeMath {
    function add(uint x, uint y) internal pure returns (uint z) {
        require((z = x + y) >= x, "ds-math-add-overflow");
    }

    function sub(uint x, uint y) internal pure returns (uint z) {
        require((z = x - y) <= x, "ds-math-sub-underflow");
    }

    function mul(uint x, uint y) internal pure returns (uint z) {
        require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
    }
}

contract math {
    using SafeMath for uint;

    function mul_test(uint a, uint b) public returns (uint) {
        return a.mul(b);
    }

    function add_test(uint a, uint b) public returns (uint) {
        return a.add(b);
    }

    function sub_test(uint a, uint b) public returns (uint) {
        return a.sub(b);
    }
}
