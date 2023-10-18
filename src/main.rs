use std::{net::{SocketAddr}, collections::{HashMap, HashSet}, time::Duration, env};

use futures::stream::FuturesUnordered;
use shutil::pipe;
use tokio::{net::{TcpListener, TcpStream, ToSocketAddrs}, io::copy_bidirectional};

pub fn get_ip_of_pod_in_namespace(namespace: &str) -> Option<HashSet<String>> {
    //this only works on linux
    let output_result = pipe(vec![
        vec!["kubectl", "describe", "-n", namespace, "pod"],
        vec!["grep", "IP:"],
        vec!["sed", "s: ::g"],
    ]);

    if let Err(err) = &output_result {
        eprintln!("failed to get k8s pod host: {:?} {:?}", err.code(), err);
        None?;
    }

    let ips = output_result.unwrap().split("\n").map(|ip| ip.replace("IP:", "").trim().to_owned()).collect::<HashSet<_>>();

    Some(ips)
}

pub struct Router;

impl Router {
    pub async fn tcp(source: impl ToSocketAddrs, destination: String) {
        let Ok(listener) = TcpListener::bind(source).await else {
            eprintln!("Failed to bind tcp");
            return;
        };
        tokio::spawn(async move {
            let destination = destination.clone();
            while let Ok((mut inbound, _)) = listener.accept().await {
                println!("connecting at host: \"{}\"", &destination);
                let Ok(mut outbound) = TcpStream::connect(destination.clone()).await else {
                    eprintln!("Failed to connect tcp");
                    continue;
                };
        
                tokio::spawn(async move {
                    let Ok(_) = copy_bidirectional(&mut inbound, &mut outbound).await else {
                        eprintln!("failed to stream tcp data"); //lol
                        return;
                    };
                });
            }
        });
    }
}

#[tokio::main]
async fn main() {
    println!("binding postgres");
    tokio::spawn(async move {
        Router::tcp("0.0.0.0:5432", env::var("DB_HOST").unwrap()).await;
    });
    println!("bound postgres");
    println!("binding redis");
    tokio::spawn(async move {
        Router::tcp("0.0.0.0:6379", env::var("REDIS_HOST").unwrap()).await;
    });
    println!("bound redis");
    loop {
        //so the application does not exit
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
