use anyhow::Result;

use orogene::Orogene;

#[async_std::main]
async fn main() -> Result<()> {
    Orogene::load().await
}
