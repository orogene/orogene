use miette::Result;
use orogene::Orogene;

#[tokio::main]
async fn main() -> Result<()> {
    Ok(Orogene::load().await?)
}
