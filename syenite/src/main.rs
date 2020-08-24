use anyhow::Result;

use syenite::Orogene;

#[async_std::main]
async fn main() -> Result<()> {
    Orogene::load().await
}
