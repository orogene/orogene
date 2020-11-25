use std::os::raw::c_char;

// #[link(name = "oronode")]
extern "C" {
    pub fn run_node(code: *mut c_char) -> i32;
}
