// full_example.sol

/*
 This is an example contract to show all the features that the
 Solang Solidity Compiler supports.
*/

contract full_example {
	// Process state
	enum State {
		Running,
		Sleeping,
		Waiting,
		Stopped,
		Zombie,
		StateCount
	}

	// Variables in contract storage
	State state;
	int32 pid;
	uint32 reaped = 3;

	// Constants
	State constant bad_state = State.Zombie;
	int32 constant first_pid = 1;

	// Our constructors
	constructor(int32 _pid) public {
		// Set contract storage
		pid = _pid;
	}

	// Reading but not writing contract storage means function
	// can be declared view
	function is_zombie_reaper() public view returns (bool) {
		/* must be pid 1 and not zombie ourselves */
		return (pid == first_pid && state != State.Zombie);
	}

	// Returning a constant does not access storage at all, so
	// function can be declared pure
	function systemd_pid() public pure returns (uint32) {
		// Note that cast is required to change sign from
		// int32 to uint32
		return uint32(first_pid);
	}

	// Some fahrenheit/celcius conversions
	function celcius2fahrenheit(int32 celcius) pure public returns (int32) {
		int32 fahrenheit = celcius * 9 / 5 + 32;

		return fahrenheit;
	}

	function fahrenheit2celcius(uint32 fahrenheit) pure public returns (uint32) {
		return (fahrenheit - 32) * 5 / 9;
	}

	// This mocks a pid state
	function get_pid_state(int64 _pid) pure private returns (State) {
		int64 n = 8;
		for (int16 i = 1; i < 100; ++i) {
			if ((i % 3) == 0) {
				n *= _pid / int64(i);
			} else {
				n /= 3;
			}
		}

		return State(n % int64(State.StateCount));
	}

	// Overloaded function with different return value!
	function get_pid_state() view private returns (uint32) {
		return reaped;
	}

	function reap_processes() public {
		int32 n = 0;

		while (n < 1000) {
			if (get_pid_state(n) == State.Zombie) {
				// reap!
				reaped += 1;
			}
			n++;
		}
	}

	function run_queue() public pure returns (uint16) {
		uint16 count = 0;
		// no initializer means its 0.
		int32 n;

		do {
			if (get_pid_state(n) == State.Waiting) {
				count++;
			}
		}
		while (++n < 1000);

		return count;
	}
}
