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
    },
};

const ENV_SUNWET: &str = "SUNWET";
const ENV_SUNWET_TOKEN: &str = "SUNWET_TOKEN";

pub fn server_url() -> Result<Uri, loga::Error> {
    let url0 = env::var(ENV_SUNWET).context_with("Missing env var", ea!(var = ENV_SUNWET))?;
    return Ok(Uri::from_str(&url0).context_with("Server URL couldn't be parsed", ea!(url = url0))?);
}

pub fn server_headers() -> Result<HashMap<String, String>, loga::Error> {
    let token = env::var(ENV_SUNWET_TOKEN).context_with("Missing env var", ea!(var = ENV_SUNWET_TOKEN))?;
    return Ok([(AUTHORIZATION.to_string(), format!("{}{}", HEADER_BEARER_PREFIX, token))].into_iter().collect());
}

pub async fn req<
    T: C2SReqTrait,
>(log: &Log, conn: &mut Conn, headers: &HashMap<String, String>, url: &Uri, req: T) -> Result<T::Resp, loga::Error> {
    return Ok(htreq::post_json::<T::Resp>(log, conn, &url.join("api"), headers, req.into(), usize::MAX).await?);
}

pub async fn req_simple<T: C2SReqTrait>(log: &Log, req0: T) -> Result<T::Resp, loga::Error> {
    let url = server_url()?;
    let headers = server_headers()?;
    let mut conn = htreq::connect(&url).await?;
    return Ok(req(log, &mut conn, &headers, &url, req0).await?);
}
