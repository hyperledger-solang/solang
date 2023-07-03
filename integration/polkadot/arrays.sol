enum Permission {
	Perm1, Perm2, Perm3, Perm4, Perm5, Perm6, Perm7, Perm8
}

/// This is a test contract which tests arrays
contract arrays {
	uint64 user_count;

	struct user {
		string name;
		bytes32 addr;
		uint64 id;
		Permission[] perms;
	}

	user[2048] users;

	mapping (bytes32 => uint64) addressToUser;

	function addUser(uint64 id, bytes32 addr, string name, Permission[] perms) public {
		user storage u = users[id];

		u.name = name;
		u.addr = addr;
		u.id = id;
		u.perms = perms;

		addressToUser[addr] = id;

		assert(id <= users.length);
	}

	function getUserById(uint64 id) public view returns (user) {
		assert(users[id].id == id);

		return users[id];
	}

	function getUserByAddress(bytes32 addr) public view returns (user) {
		uint64 id = addressToUser[addr];

		assert(users[id].id == id);

		return users[id];
	}

	function userExists(uint64 id) public view returns (bool) {
		return users[id].id == id;
	}

	function removeUser(uint64 id) public {
		bytes32 addr = users[id].addr;

		delete users[id];
		delete addressToUser[addr];
	}

	function hasPermission(uint64 id, Permission p) public view returns (bool) {
		user storage u = users[id];

		assert(u.id == id);

		for (uint32 i = 0; i < u.perms.length; i++) {
			if (u.perms[i] == p) {
				return true;
			}
		}

		return false;
	}
}
