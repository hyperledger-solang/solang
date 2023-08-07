contract c {
    uint128 var;
    modifier foo1() { uint128 x = var; _; }
    modifier foo2() { var = 2; _; }
    modifier foo3() { var = msg.value; _; }

    function bar1() foo1() public pure {}
    function bar2() foo2() public view {}
    function bar3() foo3() public {}
}
// ---- Expect: diagnostics ----
// warning: 3:31-32: local variable 'x' is unused
// error: 7:21-27: function declared 'pure' but modifier reads from state
// 	note 3:35-38: read to state
// error: 8:21-27: function declared 'view' but modifier writes to state
// 	note 4:23-26: write to state
// error: 9:21-27: function declared 'nonpayable' but modifier accesses value sent, which is only allowed for payable functions
// 	note 5:29-38: access of value sent
