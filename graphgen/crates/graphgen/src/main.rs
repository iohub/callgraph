extern crate clap;

use clap::{Arg, Command};

use env_logger;
use log;
use serde::Deserialize;
use tide::prelude::*;
use tide::Request;

#[derive(Debug, Deserialize)]
struct ParseFileReq {
    file: String,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    env_logger::init();

    let args = Command::new("graphgen")
        .arg(Arg::new("listen-addr").long("listen-addr"))
        .get_matches();

    let addr = args.get_one::<String>("listen-addr").unwrap();

    let mut app = tide::new();
    app.at("/codeindex/parse/file").post(api_parse_file);
    app.listen(addr).await?;
    Ok(())
}

async fn api_parse_file(mut req: Request<()>) -> tide::Result {
    let ParseFileReq { file } = req.body_json().await?;
    Ok(json!({
        "code": 200,
        "message": "success"
    })
    .into())
}
