abstract contract A {
	function v(int) public virtual;
}
contract C {
	function t(A a) public {
		a.v(1);
	}
}
