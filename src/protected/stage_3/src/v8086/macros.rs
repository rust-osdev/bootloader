/// Converts a segment:offset pair to a logical address
macro_rules! seg_off_to_log {
	($segment:ident, $offset:ident) => {($segment & 0xFFFF) * 16 + $offset};
}
