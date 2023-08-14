struct User { string name; uint count; }
function clear_count(User memory user) {
	user.count = 0;
}
using {clear_count} for User global;
