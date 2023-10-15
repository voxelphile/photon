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
    pub async fn database(source: impl ToSocketAddrs, destination_port: String) {
        let Ok(listener) = TcpListener::bind(source).await else {
            eprintln!("Failed to bind tcp");
            return;
        };
        tokio::spawn(async move {
            while let Ok((mut inbound, _)) = listener.accept().await {
                let destination = match env::var("PHOTON_STRATEGY").unwrap().to_lowercase().as_str() {
                    "k8s" => {
                        let Some(ips) = get_ip_of_pod_in_namespace("photon") else {
                            continue;
                        };

                        //for now, there are no database replicas, so we assert there is only one pod
                        assert_eq!(ips.len(), 1);

                        format!("{}:{}", ips.iter().next().unwrap(), destination_port)
                    },
                    "host" => {
                        format!("{}:{}", env::var("DB_IP").unwrap(), destination_port)
                    }
                    _ => panic!("invalid strategy")
                };
                println!("connecting to database at host: \"{}\"", &destination);
                let Ok(mut outbound) = TcpStream::connect(destination).await else {
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
    Router::database("0.0.0.0:5432").await;
    loop {
        //so the application does not exit
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
