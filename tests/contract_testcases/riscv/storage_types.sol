// Tested by r55/tests/solang_storage_types.rs.

type Price is uint64;

contract storage_types {
	enum Color { Red, Green, Blue }

	bool public flag = true;
	address public owner = 0x000000000000000000000000000000000000dEaD;
	int64 public signed = -42;
	bytes4 public tag = 0xcafebabe;
	Color public color = Color.Blue;
	Price public price = Price.wrap(1000);
	uint256 public big = 2**255 + 1;

	function set(bool f, address o, int64 s, bytes4 t, Color c, Price p, uint256 b) public {
		flag = f;
		owner = o;
		signed = s;
		tag = t;
		color = c;
		price = p;
		big = b;
	}

	function clear() public {
		delete flag;
		delete owner;
		delete signed;
		delete tag;
		delete color;
		delete price;
		delete big;
	}
}
