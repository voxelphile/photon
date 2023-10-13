use std::{net::{SocketAddr}, collections::HashMap};

use futures::stream::FuturesUnordered;
use tokio::{net::{TcpListener, TcpStream, ToSocketAddrs}, io::copy_bidirectional};

pub struct Router;

impl Router {
    pub async fn tcp(source: impl ToSocketAddrs, destination: impl ToSocketAddrs + Clone + Send + Sync + 'static) {
        let listener = TcpListener::bind(source).await.expect("failed to bind tcp proxy");
        tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let mut outbound = TcpStream::connect(destination.clone()).await.expect("failed to connect to tcp destination");
        
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
