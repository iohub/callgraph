extern crate clap;

use clap::{Arg, Command};

use serde::Deserialize;
use tide::prelude::*;
use tide::Request;

#[derive(Debug, Deserialize)]
struct Animal {
    name: String,
    legs: u16,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = Command::new("graphgen")
        .arg(Arg::new("listen-addr").long("listen-addr"))
        .get_matches();

    let addr = args.get_one::<String>("listen-addr").unwrap();

    let mut app = tide::new();
    app.at("/orders/shoes").post(order_shoes);
    app.listen(addr).await?;
    Ok(())
}

async fn order_shoes(mut req: Request<()>) -> tide::Result {
    let Animal { name, legs } = req.body_json().await?;
    Ok(format!("Hello, {}! I've put in an order for {} shoes", name, legs).into())
}
