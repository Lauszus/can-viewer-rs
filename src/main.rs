use futures_util::StreamExt;
use socketcan::{CanFrame, tokio::CanSocket};
use std::env;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let iface = env::args().nth(1).unwrap_or_else(|| "vcan0".into());
    let mut sock = CanSocket::open(&iface).unwrap();

    println!("Reading on {iface}");

    while let Some(res) = sock.next().await {
        match res {
            Ok(CanFrame::Data(frame)) => println!("{frame:?}"),
            Ok(CanFrame::Remote(frame)) => println!("{frame:?}"),
            Ok(CanFrame::Error(frame)) => println!("{frame:?}"),
            Err(err) => eprintln!("{err}"),
        }
    }

    Ok(())
}
