use crate::AlabasterError;

mod bridge;

pub async fn init() -> Result<(), AlabasterError> {
    bridge::init().await
}
