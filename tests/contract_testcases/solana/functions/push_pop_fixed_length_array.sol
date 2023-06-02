contract MyTest {
    function  foo1() public pure returns (uint256)  { 
        uint64[10] a; 
        a[9]  =  0x41; 
        a.push(2); 
        return  (a[9]); 
    }

    function foo2() public pure returns (uint256) {
        uint64[10] a; 
        a[9]  =  0x41; 
        a.pop(); 
        return  (a[9]);
    }
}


// ---- Expect: diagnostics ----
// error: 5:11-15: method 'push' does not exist
// error: 12:11-14: method 'pop' does not exist