use {
    super::state::{
        get_global_config,
        get_user_config,
        State,
    },
    crate::{
        interface::config::IamGrants,
        server::subsystems::oidc::get_req_session,
    },
    cookie::Cookie,
    flowcontrol::shed,
    http::{
        header::COOKIE,
        HeaderMap,
    },
    htwrap::htserve::{
        self,
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    shared::interface::{
        iam::UserIdentityId,
        wire::link::COOKIE_LINK_SESSION,
    },
};

pub enum Identity {
    Token(IamGrants),
    User(UserIdentityId),
    Link(String),
    Public,
}

// None = can't be identified = 401.
pub async fn identify_requester(
    state: &State,
    headers: &HeaderMap,
) -> Result<Option<Identity>, VisErr<loga::Error>> {
    let global_config = get_global_config(state).await.err_internal()?;
    if let Ok(got_token) = htserve::auth::get_auth_token(headers) {
        if let Some(grants) = global_config.api_tokens.get(&got_token) {
            state.log.log(loga::DEBUG, "Request user identified as token");
            return Ok(Some(Identity::Token(grants.clone())));
        }
    }
    if let Some(oidc_state) = &state.oidc_state {
        shed!{
            let Some(session) = get_req_session(&state.log, headers) else {
                break;
            };
            let Some(user) = oidc_state.sessions.get(&session).await else {
                state
                    .log
                    .log(loga::DEBUG, format!("Request has session id [{}] but no matching session found", session));
                break;
            };
            state.log.log(loga::DEBUG, format!("Request user identified as [{}]", user.0));
            return Ok(Some(Identity::User(user)));
        }
    }
    shed!{
        let Some(cookies) = headers.get(COOKIE) else {
            break;
        };
        let Ok(cookies) = cookies.to_str() else {
            break;
        };
        for c in Cookie::split_parse(cookies) {
            let Ok(c) = c else {
                continue;
            };
            if c.name() != COOKIE_LINK_SESSION {
                eprintln!("link cookie not link session: {} (want {})", c.name(), COOKIE_LINK_SESSION);
                continue;
            };
            return Ok(Some(Identity::Link(c.value().to_string())));
        }
    }
    state.log.log(loga::DEBUG, "Request user identified as public");
    return Ok(Some(Identity::Public));
}

pub async fn is_admin(state: &State, identity: &Identity) -> Result<bool, loga::Error> {
    match identity {
        Identity::Token(grants) => {
            match grants {
                IamGrants::Admin => {
                    return Ok(true);
                },
                _ => {
                    return Ok(false);
                },
            }
        },
        Identity::User(u) => {
            let user_config = get_user_config(&state, u).await?;
            match &user_config.iam_grants {
                IamGrants::Admin => {
                    return Ok(true);
                },
                _ => {
                    return Ok(false);
                },
            }
        },
        Identity::Link(_) => {
            return Ok(false);
        },
        Identity::Public => {
            return Ok(false);
        },
    }
}
