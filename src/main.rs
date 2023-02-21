use miette::Result;
use orogene::Orogene;

#[tokio::main]
async fn main() -> Result<()> {
    Orogene::load().await
}
