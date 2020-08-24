use anyhow::Result;

use syenite::Syenite;

#[async_std::main]
async fn main() -> Result<()> {
    Syenite::load().await
}
