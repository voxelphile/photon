use std::{net::{SocketAddr}, collections::HashMap, time::Duration, env};

use futures::stream::FuturesUnordered;
use shutil::pipe;
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
                let destination = match env::var("PHOTON_STRATEGY").unwrap().to_lowercase().as_str() {
                    "k8s" => {
                        //this only works on linux
                        let output_result = pipe(vec![
                            vec!["kubectl", "describe", "-n", "photon", "pod"],
                            vec!["grep", "'IP:'"],
                            vec!["head", "-1"],
                            vec!["sed", "'s: ::g'"],
                        ]);

                        if let Err(err) = output_result {
                            eprintln!("failed to get k8s pod host: {:?} {:?}", err.code(), err);
                            continue;
                        }

                        output_result.unwrap().replace("IP:", "")
                    },
                    "host" => {
                        env::var("DB_HOST").unwrap()
                    }
                    _ => panic!("invalid strategy")
                };
                println!("connecting to database at host: \"{}\"", &destination);
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
