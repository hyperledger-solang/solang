contract C {
	event E1(int);
	event E1(int, int);

	function e() public {
		emit E1();
	}
}

// ---- Expect: diagnostics ----
// error: 6:3-12: event type 'E1' has 1 fields, 0 provided
// 	note 2:8-10: definition of E1
// 	note 2:2-15: candidate event
// error: 6:3-12: event type 'E1' has 2 fields, 0 provided
// 	note 3:8-10: definition of E1
// 	note 3:2-20: candidate event
// error: 6:3-12: cannot find event with matching signature
