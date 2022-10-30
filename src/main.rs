use miette::Result;
use orogene::Orogene;

#[async_std::main]
async fn main() -> Result<()> {
    Ok(Orogene::load().await?)
}
