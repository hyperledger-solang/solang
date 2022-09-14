
event First(
	uint32 indexed a,
	bool b,
	string c
);

event Second(
	uint32 indexed a,
	string b,
	string c
);

contract MyContractEvents {
	function test() public {
		emit First(102, true, "foobar");
		emit Second(500332, "ABCD", "CAFE0123");
	}
}
