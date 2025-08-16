use {
    crate::{
        cap_fn,
        interface::config::OidcConfig,
    },
    cookie::CookieBuilder,
    flowcontrol::shed,
    http::{
        header::HOST,
        request::Parts,
        HeaderMap,
        Request,
        Response,
        Uri,
    },
    htwrap::{
        htreq,
        htserve::{
            responses::{
                body_empty,
                body_full,
                response_400,
                Body,
            },
            viserr::{
                ResultVisErr,
                VisErr,
            },
        },
        url::UriJoin,
    },
    loga::{
        ea,
        ErrContext,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    oauth2::{
        basic::{
            BasicErrorResponseType,
            BasicTokenType,
        },
        StandardRevocableToken,
    },
    openidconnect::{
        core::{
            CoreAuthDisplay,
            CoreAuthPrompt,
            CoreAuthenticationFlow,
            CoreClient,
            CoreGenderClaim,
            CoreJsonWebKey,
            CoreJsonWebKeyType,
            CoreJsonWebKeyUse,
            CoreJweContentEncryptionAlgorithm,
            CoreJwsSigningAlgorithm,
            CoreProviderMetadata,
        },
        AccessTokenHash,
        AuthorizationCode,
        ClientId,
        ClientSecret,
        CsrfToken,
        EmptyAdditionalClaims,
        EmptyExtraTokenFields,
        IdTokenFields,
        IssuerUrl,
        Nonce,
        OAuth2TokenResponse,
        PkceCodeChallenge,
        PkceCodeVerifier,
        RedirectUrl,
        RevocationErrorResponseType,
        StandardErrorResponse,
        StandardTokenIntrospectionResponse,
        StandardTokenResponse,
        TokenResponse,
    },
    rand::distributions::{
        Alphanumeric,
        DistString,
    },
    serde::Deserialize,
    shared::interface::iam::UserIdentityId,
    std::{
        borrow::Cow,
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
};

pub const COOKIE_SESSION: &str = "sunwet_session";

async fn oidc_http_client(
    log: &loga::Log,
    req: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, loga::Error> {
    let log = log.clone();
    let mut conn = htreq::connect(htreq::Limits::default(), &Uri::try_from(&req.url.to_string()).unwrap()).await?;
    let mut req1 = Request::builder();
    req1 = req1.uri(req.url.to_string());
    req1 = req1.method(match req.method {
        openidconnect::http::Method::GET => http::Method::GET,
        openidconnect::http::Method::POST => http::Method::POST,
        openidconnect::http::Method::HEAD => http::Method::HEAD,
        _ => panic!(),
    });
    req1 = req1.header(HOST, req.url.host_str().unwrap_or_default());
    for (k, v) in req.headers {
        let Some(k) = k else {
            panic!();
        };
        req1 = req1.header(k.to_string(), http::HeaderValue::from_bytes(v.as_bytes()).unwrap());
    }
    let req1 = req1.body(body_full(req.body)).unwrap();
    let (code, headers, continue_) = htreq::send(&log, htreq::Limits::default(), &mut conn, req1).await?;
    let body = htreq::receive(htreq::Limits::default(), continue_).await?;
    return Ok(openidconnect::HttpResponse {
        status_code: openidconnect::http::StatusCode::from_u16(code.as_u16()).unwrap(),
        headers: {
            let mut headers1 = openidconnect::http::HeaderMap::new();
            for (k, v) in headers {
                let Some(k) = k else {
                    panic!();
                };
                headers1.append(
                    openidconnect::http::HeaderName::from_bytes(k.as_ref()).unwrap(),
                    openidconnect::http::HeaderValue::from_bytes(v.as_bytes()).unwrap(),
                );
            }
            headers1
        },
        body: body,
    });
}

struct OidcPreSession {
    original_url: Uri,
    pkce_verifier: Mutex<Option<PkceCodeVerifier>>,
    nonce: Nonce,
}

pub struct OidcState {
    log: loga::Log,
    client: openidconnect
    ::Client<
        EmptyAdditionalClaims,
        CoreAuthDisplay,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
        CoreJsonWebKeyType,
        CoreJsonWebKeyUse,
        CoreJsonWebKey,
        CoreAuthPrompt,
        StandardErrorResponse<BasicErrorResponseType>,
        StandardTokenResponse<
            IdTokenFields<
                EmptyAdditionalClaims,
                EmptyExtraTokenFields,
                CoreGenderClaim,
                CoreJweContentEncryptionAlgorithm,
                CoreJwsSigningAlgorithm,
                CoreJsonWebKeyType,
            >,
            BasicTokenType,
        >,
        BasicTokenType,
        StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
        StandardRevocableToken,
        StandardErrorResponse<RevocationErrorResponseType>,
    >,
    pre_sessions: Cache<String, Arc<OidcPreSession>>,
    pub(crate) sessions: Cache<String, UserIdentityId>,
}

pub async fn new_state(log: &Log, oidc_config: OidcConfig) -> Result<OidcState, loga::Error> {
    let log = log.fork(ea!(subsystem = "oidc"));
    let client =
        CoreClient::from_provider_metadata(
            CoreProviderMetadata::discover_async(IssuerUrl::new(oidc_config.provider_url)?, cap_fn!((r)(log) {
                return oidc_http_client(&log, r).await.map_err(|e| std::io::Error::other(e.to_string()));
            })).await?,
            ClientId::new(oidc_config.client_id.clone()),
            oidc_config.client_secret.as_ref().map(|s| ClientSecret::new(s.clone())),
        );
    return Ok(OidcState {
        log: log,
        client: client,
        pre_sessions: Cache::builder().max_capacity(10).time_to_live(Duration::from_secs(60 * 10)).build(),
        sessions: Cache::builder().time_to_idle(Duration::from_secs(60 * 60 * 24 * 7)).build(),
    });
}

pub async fn handle_oidc(state: &OidcState, head: Parts) -> Result<Response<Body>, VisErr<loga::Error>> {
    let log = state.log.clone();
    let Some(query) = head.uri.query() else {
        return Ok(response_400("Missing query"));
    };

    // Try handling received token from completed oidc
    shed!{
        #[derive(Deserialize)]
        struct Params {
            code: String,
            state: String,
        }

        let Ok(params) = serde_urlencoded::from_str::<Params>(query) else {
            break;
        };
        let Some(pre_session_state) = state.pre_sessions.remove(&params.state).await else {
            log.log_with(loga::DEBUG, "Missing pre-session state for state", ea!(state = params.state));
            break;
        };
        let pkce_verifier = pre_session_state.pkce_verifier.lock().unwrap().take().unwrap();
        let token_response =
            state
                .client
                .exchange_code(AuthorizationCode::new(params.code))
                .set_pkce_verifier(pkce_verifier)
                .request_async(cap_fn!((r)(log) {
                    return oidc_http_client(&log, r).await.map_err(|e| std::io::Error::other(e.to_string()));
                }))
                .await
                .context("Error exchanging token from callback with OIDC server")
                .err_internal()?;
        let id_token = token_response.id_token().context("OIDC server response missing ID token").err_internal()?;
        let claims =
            id_token
                .claims(&state.client.id_token_verifier(), &pre_session_state.nonce)
                .context("Error getting claims from OIDC server response")
                .err_internal()?;
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash =
                AccessTokenHash::from_token(
                    token_response.access_token(),
                    &id_token.signing_alg().context("Error getting signing alg from server response").err_internal()?,
                )
                    .context("Error hashing access token in server response")
                    .err_internal()?;
            if actual_access_token_hash != *expected_access_token_hash {
                log.log_with(
                    loga::DEBUG,
                    "Access token hash mismatch",
                    ea!(want = *expected_access_token_hash, got = actual_access_token_hash),
                );
                break;
            }
        }
        let session_cookie = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        state.sessions.insert(session_cookie.clone(), UserIdentityId(claims.subject().to_string())).await;
        return Ok(
            http::Response::builder()
                .status(http::StatusCode::TEMPORARY_REDIRECT)
                .header(
                    http::header::SET_COOKIE,
                    CookieBuilder::new(COOKIE_SESSION, session_cookie)
                        .http_only(true)
                        .secure(true)
                        .permanent()
                        .build()
                        .to_string(),
                )
                .header(http::header::LOCATION, &pre_session_state.original_url.to_string())
                .body(body_empty())
                .unwrap(),
        );
    };

    // Start a new auth flow
    #[derive(Deserialize)]
    struct Params {
        #[serde(with = "http_serde::uri")]
        url: Uri,
    }

    let params = match serde_urlencoded::from_str::<Params>(query) {
        Ok(p) => p,
        Err(e) => {
            log.log_err(
                loga::DEBUG,
                e.context(format!("Received new auth request with invalid query string: [{}]", query)),
            );
            return Ok(response_400("Invalid query params"));
        },
    };
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) =
        state
            .client
            .authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random)
            .set_redirect_uri(
                Cow::Owned(
                    RedirectUrl::new(params.url.join("../oidc").to_string())
                        .context("Error creating redirect url from current state url")
                        .err_internal()?,
                ),
            )
            .set_pkce_challenge(pkce_challenge)
            .url();
    state.pre_sessions.insert(csrf_token.secret().clone(), Arc::new(OidcPreSession {
        original_url: params.url,
        pkce_verifier: Mutex::new(Some(pkce_verifier)),
        nonce: nonce,
    })).await;
    return Ok(
        http::Response::builder()
            .status(http::StatusCode::TEMPORARY_REDIRECT)
            .header(http::header::LOCATION, auth_url.to_string())
            .body(body_empty())
            .unwrap(),
    );
}

pub fn get_req_session<'a>(log: &Log, headers: &HeaderMap) -> Option<String> {
    let Some(v) = headers.get(http::header::COOKIE).and_then(|c| c.to_str().ok()) else {
        eprintln!("no cookie header");
        return None;
    };
    for cookie in cookie::Cookie::split_parse(v) {
        let cookie = match cookie {
            Ok(c) => c,
            Err(e) => {
                log.log_err(loga::DEBUG, e.context("Error parsing received cookie"));
                continue;
            },
        };
        if cookie.name() == COOKIE_SESSION {
            return Some(cookie.value().to_string());
        }
        eprintln!("cookie not session: {} v {}", cookie.name(), COOKIE_SESSION);
    }
    eprintln!("no session header");
    return None;
}

pub async fn handle_logout(
    state: &OidcState,
    log: &Log,
    head: Parts,
) -> Result<Response<Body>, VisErr<loga::Error>> {
    if let Some(session) = get_req_session(log, &head.headers) {
        state.sessions.remove(&session).await;
    }

    #[derive(Deserialize)]
    struct Params {
        url: String,
    }

    let Some(query) = head.uri.query() else {
        return Ok(response_400("Missing query"));
    };
    let params = match serde_urlencoded::from_str::<Params>(query) {
        Ok(p) => p,
        Err(e) => {
            log.log_err(
                loga::DEBUG,
                e.context(format!("Received logout request with invalid query string: [{}]", query)),
            );
            return Ok(response_400("Invalid query params"));
        },
    };
    return Ok(
        http::Response::builder()
            .status(http::StatusCode::TEMPORARY_REDIRECT)
            .header(http::header::LOCATION, params.url)
            .body(body_empty())
            .unwrap(),
    );
}
