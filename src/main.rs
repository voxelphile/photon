use std::{net::{SocketAddr}, collections::HashMap, time::Duration, env};

use futures::stream::FuturesUnordered;
use tokio::{net::{TcpListener, TcpStream, ToSocketAddrs}, io::copy_bidirectional};

pub struct Router;

impl Router {
    pub async fn database<>(source: impl ToSocketAddrs) {
        let Ok(listener) = TcpListener::bind(source).await else {
            eprintln!("Failed to bind tcp");
            return;
        };
        tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let destination = match env::var("PROTON_STRATEGY").unwrap().to_lowercase().as_str() {
                    "k8s" => {
                        //this only works on linux
                        let output = tokio::process::Command::new("kubectl describe -n photon pod | grep 'IP:' | head -1 | sed 's: ::g").spawn().unwrap().wait_with_output().await.unwrap();    
                        String::from_utf8(output.stdout).unwrap().replace("IP:", "")
                    },
                    "host" => {
                        env::var("DB_HOST").unwrap()
                    }
                    _ => panic!("invalid strategy")
                };
                let Ok(mut outbound) = TcpStream::connect(destination).await else {
                    eprintln!("Failed to connect tcp");
                    return;
                };
        
                tokio::spawn(async move {
                    let Ok(_) = copy_bidirectional(&mut inbound, &mut outbound).await else {
                        eprintln!("failed to stream tcp data");
                        return;
                    };
                });
            }
        });
    }
}



#[tokio::main]
async fn main() {
    Router::database("0.0.0.0:5432").await;
    loop {
        //so the application does not exit
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
