contract aborting {
    constructor() {
        revert("bar");
    }

    function never() public pure {}
}

contract runner {
    function test() public {
        try new aborting() returns (aborting a) {
            // new succeeded; a holds the a reference to the new contract
        } catch Error(string x) {
            if (x == "bar") {
                // "bar" revert or require was executed
            }
        } catch (bytes raw) {
            // if no error string could decoding, we end up here with the raw data
        }
    }
}
