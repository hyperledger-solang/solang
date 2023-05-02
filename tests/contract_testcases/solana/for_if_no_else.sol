function foo() returns (int) {
	for (int i=0; i< 10; i++)
		if (i > 0) 
			return 1;

	return 2;
}



// ---- Expect: diagnostics ----
// warning: 1:1-29: function can be declared 'pure'
