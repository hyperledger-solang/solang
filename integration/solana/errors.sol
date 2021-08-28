contract errors {
    function do_revert(bool yes) pure public returns (int) {
        if (yes) {
            revert("Do the revert thing");
        } else {
            return 3124445;
        }
    }
}