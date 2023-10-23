#![feature(async_closure)]
use std::{
    collections::{HashMap, HashSet},
    env,
    hash::Hash,
    net::SocketAddr,
    time::Duration,
};

use futures::stream::FuturesUnordered;
use hyper::{
    header::HeaderValue, server::conn::Http, service::service_fn, HeaderMap, Method, Response,
    StatusCode,
};
use shutil::pipe;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

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

    let ips = output_result
        .unwrap()
        .split("\n")
        .map(|ip| ip.replace("IP:", "").trim().to_owned())
        .collect::<HashSet<_>>();

    Some(ips)
}

pub type Host = &'static str;
pub type Destination = &'static str;

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
    pub async fn http<const N: usize>(
        source: impl ToSocketAddrs,
        routes: [(Host, Destination); N],
    ) {
        let Ok(listener) = TcpListener::bind(source).await else {
            eprintln!("Failed to bind tcp");
            return;
        };
        tokio::spawn(async move {
            while let Ok((inbound, _)) = listener.accept().await {
                if let Err(err) = Http::new()
                .serve_connection(inbound, service_fn(async move |req: hyper::Request<hyper::Body>| -> Result<Response<String>, hyper::Error> {
                    let Some((_, destination)) = routes.iter().find(|(host, _)| *host == req.headers().get("Host").unwrap().to_str().unwrap()) else {
                        let mut resp = hyper::Response::new(String::default());
                        resp.headers_mut().insert(hyper::header::LOCATION, HeaderValue::from_str("https://voxelphile.com/").unwrap());
                        return Ok(resp);
                    };

                    let destination = destination.to_owned();

                    let proxy_client = reqwest::Client::new();
                    
                    let proxy_req = match req.method() {
                        &reqwest::Method::POST => proxy_client.post(destination),
                        &reqwest::Method::GET => proxy_client.get(destination),
                        &reqwest::Method::PUT => proxy_client.put(destination),
                        &reqwest::Method::PATCH => proxy_client.patch(destination),
                        &reqwest::Method::DELETE => proxy_client.delete(destination),
                        _ => {
                            let mut resp = hyper::Response::new(String::default());
                            *resp.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                            return Ok(resp);
                        }
                    };

                    let mut proxy_headers = reqwest::header::HeaderMap::new();

                    for (k, v) in req.headers() {
                        proxy_headers.insert(k, v.clone());
                    }

                    let Ok(req_body) = String::from_utf8(hyper::body::to_bytes(req.into_body()).await?.to_vec()) else {
                        let mut resp = hyper::Response::default();
                        *resp.status_mut() = StatusCode::BAD_REQUEST;
                        return Ok(resp);
                    };

                    let Ok(proxy_resp) = proxy_req.headers(proxy_headers).body(req_body).send().await else {
                        let mut resp = hyper::Response::default();
                        *resp.status_mut() = StatusCode::BAD_GATEWAY;
                        return Ok(resp);
                    };

                    let mut resp = hyper::Response::new(String::new());

                    *resp.status_mut() = proxy_resp.status();

                    let mut resp_headers = hyper::HeaderMap::new();

                    for (k, v) in proxy_resp.headers() {
                        resp_headers.insert(k, v.clone());
                    }

                    *resp.headers_mut() = resp_headers;

                    *resp.body_mut() = proxy_resp.text().await.unwrap_or_default();

                    Ok(resp)
                }))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
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
