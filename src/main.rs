use std::{net::{SocketAddr}, collections::HashMap};

use futures::stream::FuturesUnordered;
use tokio::{net::{TcpListener, TcpStream, ToSocketAddrs}, io::copy_bidirectional};

pub struct Router;

impl Router {
    pub async fn tcp(source: impl ToSocketAddrs, destination: impl ToSocketAddrs + Clone + Send + Sync + 'static) {
        let Ok(listener) = TcpListener::bind(source).await else {
            eprintln!("Failed to bind tcp");
            return;
        };
        tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let Ok(mut outbound) = TcpStream::connect(destination.clone()).await else {
                    eprintln!("Failed to connect tcp");
                    return;
                };
        
                tokio::spawn(async move {
                    copy_bidirectional(&mut inbound, &mut outbound).await.expect("failed to stream tcp data");
                });
            }
        });
    }
}

#[tokio::main]
async fn main() {
    Router::tcp("0.0.0.0:5432", "34.118.225.0:5432").await;
}
