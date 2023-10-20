// SPDX-License-Identifier: MIT

// Interface for an operator that performs an operation on two int32 values.
interface Operator {
    function performOperation(int32 a, int32 b) external returns (int32);
}

contract Ferqu {
    Operator public operator;

    // Constructor that takes a boolean parameter 'doAdd'.
    constructor(bool doAdd) {
        if (doAdd) {
            operator = new Adder();
        } else {
            operator = new Subtractor();
        }
    }

    // Function to calculate the result of the operation performed by the chosen operator.
    function calculate(int32 a, int32 b) public returns (int32) {
        return operator.performOperation(a, b);
    }
}

// Contract for addition, implementing the 'Operator' interface.
contract Adder is Operator {
    function performOperation(int32 a, int32 b) public pure override returns (int32) {
        return a + b;
    }
}

// Contract for subtraction, implementing the 'Operator' interface.
contract Subtractor is Operator {
    function performOperation(int32 a, int32 b) public pure override returns (int32) {
        return a - b;
    }
}
