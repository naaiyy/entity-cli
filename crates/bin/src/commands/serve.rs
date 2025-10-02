use anyhow::Result;
use api::build_router;
use axum::serve;
use tokio::net::TcpListener;

use crate::cli::ServeCmd;

pub fn run(ServeCmd { addr }: ServeCmd) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let router = build_router().await?;
        let listener = TcpListener::bind(&addr).await?;
        serve(listener, router.into_make_service()).await?;
        Ok::<(), anyhow::Error>(())
    })
}
