pragma solidity 0.5.0;

contract SolangTest {
    uint value1;
    uint value2;
    
    
    /*For a function declared without pure or view:
    a. If no statement or expression or call to another function reads from storage, then the warning: "function can be declared pure" should be given*/
    function test1A(uint _a, uint _b) public returns(uint) {
        uint a = _a;
        uint b = _b;
        return a + b;
    }
    
    /*For a function declared without pure or view:
    b. If no statement or expression or call to another function writes to storage, then the warning: "function can be declared view should be given.*/
    function test1B() public returns(uint) {
        return value1;
    }
    
    /*For a function declared without pure or view:
    c. no warning or error should be produced*/
    function test1C(uint _value1, uint _value2) public returns(uint) {
        value1 = _value1;
        value2 = _value2;
        return value1 + value2;
    }
    
    /*If a function is declared pure:
    For each statement, expression or function call in the body of the function that reads or writes from contract store, 
    an error should be produced with the correct location of the problem. "function declared pure but a = 1 writes to contract storage"*/
    function test2(uint _value1) public pure {
        value1 = _value1;
    }
   
    /*If a function is declared view
    For each statement, expression or function call in the body of the function that writes to contract store, 
    an error should be produced with the correct location of the problem. "function declared view but a = 1 writes to contract storage*/
    function test3(uint _value2) public view returns(uint) {
        value2 = _value2;
        return value2;
    }
    
}