contract C {
    address[] private referralsKey;

    function getReferralsByAddress()view public   returns(address[] memory referralsKeyList,uint) {
        uint length = referralsKey.length;
	// the length of the array and the array itself are loaded from
	// storage slot 0. Ensure they are not merged by dead_storage.
        return (referralsKey,length);
    }
}

// ---- Expect: diagnostics ----
