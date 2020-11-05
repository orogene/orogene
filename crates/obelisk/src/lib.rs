use std::os::raw::c_char;

extern "C" {
    pub fn run_node(code: *const c_char) -> i32;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let ci = ci_info::is_ci();
        if !ci {
            {
                // TODO!
            }
        }
    }
}
