// Structs and fixed-size arrays in calldata, return data and storage.
// Tested by r55/tests/solang_static_types.rs.

type Amount is uint128;

contract static_types {
	enum Kind { None, Small, Large }

	struct Point {
		int32 x;
		int32 y;
	}

	struct Shape {
		Kind kind;
		Point origin;
		Point[2] corners;
		bytes4 tag;
		address owner;
		Amount amount;
		bool visible;
	}

	// slots 0..10: one per value inside the struct
	Shape shape;
	// slots 11..16: grid[i][j] is slot 11 + 3 * i + j
	uint16[3][2] grid;

	function echoShape(Shape memory s) public pure returns (Shape memory) {
		return s;
	}

	function echoGrid(uint16[3][2] memory g) public pure returns (uint16[3][2] memory) {
		return g;
	}

	function echoNarrow(uint8 a, int8 b, uint24 c, int128 d, bytes1 e, bool f)
		public pure returns (uint8, int8, uint24, int128, bytes1, bool)
	{
		return (a, b, c, d, e, f);
	}

	function setShape(Shape memory s) public {
		shape = s;
	}

	function getShape() public view returns (Shape memory) {
		return shape;
	}

	function setOrigin(int32 x, int32 y) public {
		shape.origin = Point(x, y);
	}

	function originX() public view returns (int32) {
		return shape.origin.x;
	}

	function setGrid(uint16[3][2] memory g) public {
		grid = g;
	}

	function getGrid() public view returns (uint16[3][2] memory) {
		return grid;
	}

	function setCell(uint i, uint j, uint16 v) public {
		grid[i][j] = v;
	}

	function cell(uint i, uint j) public view returns (uint16) {
		return grid[i][j];
	}

	function clear() public {
		delete shape;
		delete grid;
	}
}
