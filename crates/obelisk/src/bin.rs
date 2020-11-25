use obelisk::run_node;
use std::env;
use std::ffi::CString;

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let filepath = CString::new(args[1].clone()).expect("CString failed");
    let ptr = filepath.into_raw();

    unsafe {
        run_node(ptr);
        let _ = CString::from_raw(ptr);
    }
}
