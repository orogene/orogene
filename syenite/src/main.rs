use oro_diagnostics::DiagnosticResult;
use syenite::Syenite;

#[async_std::main]
async fn main() -> DiagnosticResult<()> {
    Ok(Syenite::load().await?)
}
