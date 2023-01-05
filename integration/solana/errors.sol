contract errors {
    function do_revert(bool yes) pure public returns (int) {
        if (yes) {
            print("Going to revert");
            revert("Do the revert thing");
        } else {
            return 3124445;
        }
    }
}