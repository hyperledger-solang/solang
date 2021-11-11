
event First(
	int indexed a,
	bool b,
	string c
);

event Second(
	int indexed a,
	bytes4 b,
	bytes c
);

contract events {
	function test() public {
		emit First(102, true, "foobar");
		emit Second(500332, "ABCD", hex"CAFE0123");
	}
}
