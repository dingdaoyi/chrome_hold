use std::error::Error;
use hrome_hold::start;

#[tokio::main]
async fn main()->Result<(),Box<dyn Error>> {
    start().await?;
    Ok(())
}

