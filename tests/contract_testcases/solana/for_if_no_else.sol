function foo() returns (int) {
	for (int i=0; i< 10; i++)
		if (i > 0) 
			return 1;

	return 2;
}



// ----
// warning (0-28): function can be declared 'pure'
