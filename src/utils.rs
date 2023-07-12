pub fn simple_hash(s: &str) -> u64 {
    let mut ret: u64 = 0;
    for c in s.chars() {
        ret += c as u64;
        ret <<= 4;
        ret |= c as u64;
    }
    ret
}
