/// SPDX-License-Identifier: Apache-2.0

interface IPause {
    function paused() external view returns (bool);
}

contract increment_with_pause {
    IPause private instance pauseContract;
    uint32 private instance count;

    error PausedError();

    constructor(IPause _pause) {
        pauseContract = _pause;
    }

    function increment() public returns (uint32) {
        if (pauseContract.paused()) {
            revert PausedError();
        }

        count += 1;
        return count;
    }
}

contract pause {
    bool private instance _isPaused;

    function paused() public view returns (bool) {
        return _isPaused;
    }

    function set(bool p) public {
        _isPaused = p;
    }
}