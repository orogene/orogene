use orogene::Orogene;
use oro_common::{miette::Result, smol};

fn main() -> Result<()> {
    smol::block_on(Orogene::load())?;
    Ok(())
}
