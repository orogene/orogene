use miette::Result;
use orogene::Orogene;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[async_std::main]
async fn main() -> Result<()> {
    Orogene::load().await
}
