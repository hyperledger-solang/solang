contract aborting {
    function abort() public returns (int32, bool) {
        revert("bar");
    }
}

contract runner {
    function test() public {
        aborting abort = new aborting();

        try abort.abort() returns (int32 a, bool b) {
            // call succeeded; return values are in a and b
        } catch Error(string x) {
            if (x == "bar") {
                // "bar" reason code was provided through revert() or require()
            }
        } catch (bytes raw) {
            // if no error string could decoding, we end up here with the raw data
        }
    }
}
