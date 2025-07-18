use {
    http::{
        header::AUTHORIZATION,
        Uri,
    },
    htwrap::{
        constants::HEADER_BEARER_PREFIX,
        htreq::{
            self,
            Conn,
        },
        url::UriJoin,
    },
    loga::{
        ea,
        Log,
        ResultContext,
    },
    shared::interface::wire::C2SReqTrait,
    std::{
        collections::HashMap,
        env,
        str::FromStr,
        time::Duration,
        usize,
    },
};

pub const ENV_SUNWET: &str = "SUNWET";
pub const ENV_SUNWET_TOKEN: &str = "SUNWET_TOKEN";

pub fn server_url() -> Result<Uri, loga::Error> {
    let url0 = env::var(ENV_SUNWET).context_with("Missing env var", ea!(var = ENV_SUNWET))?;
    return Ok(Uri::from_str(&url0).context_with("Server URL couldn't be parsed", ea!(url = url0))?);
}

pub fn server_headers() -> Result<HashMap<String, String>, loga::Error> {
    let token = env::var(ENV_SUNWET_TOKEN).context_with("Missing env var", ea!(var = ENV_SUNWET_TOKEN))?;
    return Ok([(AUTHORIZATION.to_string(), format!("{}{}", HEADER_BEARER_PREFIX, token))].into_iter().collect());
}

pub fn http_limits() -> htreq::Limits {
    return htreq::Limits {
        resolve_time: Duration::from_secs(60),
        connect_time: Duration::from_secs(60),
        read_header_time: Duration::from_secs(300),
        read_body_time: Duration::from_secs(300),
        read_body_size: usize::MAX,
    };
}

pub async fn req<
    T: C2SReqTrait,
>(log: &Log, conn: &mut Conn, headers: &HashMap<String, String>, url: &Uri, req: T) -> Result<T::Resp, loga::Error> {
    return Ok(
        htreq::post_json::<T::Resp>(log, http_limits(), conn, &url.join("api"), headers, req.into()).await?,
    );
}

pub async fn req_simple<T: C2SReqTrait>(log: &Log, req0: T) -> Result<T::Resp, loga::Error> {
    let url = server_url()?;
    let headers = server_headers()?;
    let mut conn = htreq::connect(http_limits(), &url).await?;
    return Ok(req(log, &mut conn, &headers, &url, req0).await?);
}
