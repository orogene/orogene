use oro_diagnostics::DiagnosticResult;
use orogene::Orogene;

#[async_std::main]
async fn main() -> DiagnosticResult<()> {
    Ok(Orogene::load().await?)
}
