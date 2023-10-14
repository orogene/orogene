#[cfg(target_os = "macos")]
mod build_macos;

fn main() {
    #[cfg(target_os = "macos")]
    build_macos::build();
}
