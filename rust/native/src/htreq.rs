use std::{
    collections::HashMap,
    str::FromStr,
    net::{
        IpAddr,
        Ipv6Addr,
        Ipv4Addr,
    },
    sync::OnceLock,
};
use chrono::{
    Duration,
};
use futures::future::join_all;
use hickory_resolver::config::LookupIpStrategy;
use http_body_util::{
    Limited,
    BodyExt,
    Full,
};
use hyper::{
    body::{
        Bytes,
    },
    client::conn::http1::{
        Connection,
        SendRequest,
    },
    Request,
    Uri,
};
use hyper_rustls::{
    HttpsConnectorBuilder,
    ConfigBuilderExt,
};
use loga::{
    ea,
    DebugDisplay,
    ResultContext,
};
use rand::{
    seq::SliceRandom,
    thread_rng,
};
use rustls::ClientConfig;
use tokio::{
    select,
    time::sleep,
    sync::{
        mpsc,
    },
};
use tower_service::Service;
use crate::{
    ta_res,
    util::{
        Flag,
        Log,
    },
};

pub fn rustls_client_config() -> rustls::ClientConfig {
    static S: OnceLock<rustls::ClientConfig> = OnceLock::new();
    return S.get_or_init(move || {
        ClientConfig::builder()
            .with_native_roots()
            .context("Error loading native roots")
            .unwrap()
            .with_no_client_auth()
    }).clone();
}

pub struct Conn {
    inner: Option<
        (
            SendRequest<Full<Bytes>>,
            Connection<
                hyper_rustls::MaybeHttpsStream<hyper_util::rt::tokio::TokioIo<tokio::net::TcpStream>>,
                Full<Bytes>,
            >,
        ),
    >,
}

pub enum HostPart {
    Ip(IpAddr),
    Name(String),
}

pub fn uri_parts(url: &Uri) -> Result<(String, HostPart, u16), loga::Error> {
    let host = url.host().context("Url is missing host")?;
    if host.is_empty() {
        return Err(loga::err("Host portion of url is empty"));
    }
    let host = if host.as_bytes()[0] as char == '[' {
        HostPart::Ip(
            IpAddr::V6(
                Ipv6Addr::from_str(
                    &String::from_utf8(
                        host.as_bytes()[1..]
                            .split_last()
                            .context("URL ipv6 missing ending ]")?
                            .1
                            .iter()
                            .cloned()
                            .collect(),
                    ).unwrap(),
                ).context("Invalid ipv6 address in URL")?,
            ),
        )
    } else if host.as_bytes().iter().all(|b| (*b as char) == '.' || ('0' ..= '9').contains(&(*b as char))) {
        HostPart::Ip(IpAddr::V4(Ipv4Addr::from_str(host).context("Invalid ipv4 address in URL")?))
    } else {
        HostPart::Name(host.to_string())
    };
    let scheme = url.scheme().context("Url is missing scheme")?.to_string();
    let port = match url.port_u16() {
        Some(p) => p,
        None => match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => return Err(loga::err("Only http/https urls are supported")),
        },
    };
    return Ok((scheme, host, port));
}

pub async fn send(
    log: &Log,
    conn: &mut Conn,
    max_size: usize,
    max_time: Duration,
    req: Request<Full<Bytes>>,
) -> Result<Vec<u8>, loga::Error> {
    let Some((mut conn_send, mut conn_bg)) = conn.inner.take() else {
        return Err(loga::err("Connection already lost"));
    };
    let method = req.method().to_string();
    let url = req.uri().to_string();
    let read = async move {
        let work = conn_send.send_request(req);
        let resp = select!{
            _ =& mut conn_bg => {
                return Err(loga::err("Connection failed while sending request"));
            }
            r = work => r,
        }.context("Error sending request")?;
        let status = resp.status();
        let headers = resp.headers().clone();
        let work = Limited::new(resp.into_body(), max_size).collect();
        let body = select!{
            _ =& mut conn_bg => {
                return Err(loga::err("Connection failed while reading body"));
            }
            r = work => r,
        }.map_err(|e| loga::err_with("Error reading response", ea!(err = e)))?.to_bytes().to_vec();
        return Ok((status, headers, body, conn_send, conn_bg));
    };
    let (status, headers, body, conn_send, conn_bg) = select!{
        _ = sleep(max_time.to_std().unwrap()) => {
            return Err(loga::err("Timeout waiting for response from server"));
        }
        x = read => x ?,
    };
    conn.inner = Some((conn_send, conn_bg));
    log.log_with(
        Flag::Debug,
        "Receive",
        ea!(
            method = method,
            url = url,
            status = status,
            headers = headers.dbg_str(),
            body = String::from_utf8_lossy(&body)
        ),
    );
    if !status.is_success() {
        return Err(
            loga::err_with(
                "Server returned error response",
                ea!(status = status, body = String::from_utf8_lossy(&body)),
            ),
        );
    }
    return Ok(body);
}

/// Creates a new HTTPS/HTTP connection with default settings.  `base_uri` is just
/// used for schema, host, and port.
pub async fn new_conn(base_url: &Uri) -> Result<Conn, loga::Error> {
    let log = &Log::new().fork(ea!(url = base_url));
    let (scheme, host, port) = uri_parts(base_url).stack_context(log, "Incomplete url")?;
    let mut ipv4s = vec![];
    let mut ipv6s = vec![];
    let host = match host {
        HostPart::Ip(i) => {
            match i {
                IpAddr::V4(_) => ipv4s.push(i),
                IpAddr::V6(_) => ipv6s.push(i),
            }
            i.to_string()
        },
        HostPart::Name(host) => {
            let (hickory_config, mut hickory_options) =
                hickory_resolver
                ::system_conf
                ::read_system_conf().stack_context(log, "Error reading system dns resolver config for http request")?;
            hickory_options.ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
            for ip in hickory_resolver::TokioAsyncResolver::tokio(hickory_config, hickory_options)
                .lookup_ip(&format!("{}.", host))
                .await
                .stack_context(log, "Failed to look up lookup host ip addresses")? {
                match ip {
                    std::net::IpAddr::V4(_) => {
                        ipv4s.push(ip);
                    },
                    std::net::IpAddr::V6(_) => {
                        ipv6s.push(ip);
                    },
                }
            }
            host
        },
    };
    {
        let mut r = thread_rng();
        ipv4s.shuffle(&mut r);
        ipv6s.shuffle(&mut r);
    }
    let mut bg = vec![];
    let (found_tx, mut found_rx) = mpsc::channel(1);
    for ips in [ipv6s, ipv4s] {
        bg.push({
            let found_tx = found_tx.clone();
            let scheme = &scheme;
            let host = &host;
            async move {
                ta_res!(());
                let mut errs = vec![];
                for ip in &ips {
                    let connect = async {
                        return Ok(
                            HttpsConnectorBuilder::new()
                                .with_tls_config(rustls_client_config())
                                .https_or_http()
                                .with_server_name(host.to_string())
                                .enable_http1()
                                .build()
                                .call(Uri::from_str(&format!("{}://{}:{}", scheme, match ip {
                                    IpAddr::V4(i) => i.to_string(),
                                    IpAddr::V6(i) => format!("[{}]", i),
                                }, port)).unwrap())
                                .await
                                .map_err(
                                    |e| loga::err_with(
                                        "Connection failed",
                                        ea!(err = e.to_string(), dest_addr = ip, host = host, port = port),
                                    ),
                                )?,
                        );
                    };
                    let res = select!{
                        _ = sleep(Duration::seconds(10).to_std().unwrap()) => Err(loga::err("Timeout connecting")),
                        res = connect => res,
                    };
                    match res {
                        Ok(conn) => {
                            _ =
                                found_tx.try_send(
                                    Conn {
                                        inner: Some(
                                            hyper::client::conn::http1::handshake(conn)
                                                .await
                                                .context("Error completing http handshake")?,
                                        ),
                                    },
                                );
                            return Ok(());
                        },
                        Err(e) => {
                            errs.push(e);
                        },
                    }
                }
                return Err(
                    loga::agg_err_with(
                        "Couldn't establish a connection to any ip in version set",
                        errs,
                        ea!(ips = ips.dbg_str()),
                    ),
                );
            }
        });
    }
    let results = select!{
        results = join_all(bg) => results,
        found = found_rx.recv() => {
            return Ok(found.unwrap());
        }
    };
    if results.is_empty() {
        return Err(log.err("No addresses found when looking up host"));
    }
    let mut failed = vec![];
    for res in results {
        match res {
            Ok(_) => {
                let found = found_rx.recv().await.unwrap();
                return Ok(found);
            },
            Err(e) => {
                failed.push(e);
            },
        }
    }
    return Err(log.agg_err("Unable to connect to host", failed));
}

pub async fn post(
    log: &Log,
    conn: &mut Conn,
    url: impl AsRef<str>,
    headers: &HashMap<String, String>,
    body: Vec<u8>,
    max_size: usize,
) -> Result<Vec<u8>, loga::Error> {
    let url = url.as_ref();
    let url = Uri::from_str(url).context_with("URI couldn't be parsed", ea!(url = url))?;
    let req = Request::builder();
    let mut req = req.method("POST").uri(url.clone());
    for (k, v) in headers.iter() {
        req = req.header(k, v);
    }
    log.log_with(
        Flag::Debug,
        "Send",
        ea!(method = "POST", url = url, headers = req.headers_ref().dbg_str(), body = String::from_utf8_lossy(&body)),
    );
    return Ok(
        send(log, conn, max_size, Duration::seconds(10), req.body(Full::new(Bytes::from(body))).unwrap())
            .await
            .context_with("Error sending POST", ea!(url = url))?,
    );
}
