use async_trait::async_trait;
use oro_diagnostics::DiagnosticResult as Result;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
