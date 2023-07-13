
contract caller {
    function doThis(int64 a) public pure returns (int64) {
        return a + 2;
    }

    function doThat(int32 b) public pure returns (int32) {
        return b + 3;
    }

    function do_call() pure public returns (int64, int32) {
        return (this.doThis(5), this.doThat(3));
    }
}