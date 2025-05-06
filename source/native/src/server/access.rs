use {
    super::{
        state::{
            get_global_config,
            get_user_config,
            State,
        },
    },
    crate::{
        interface::config::IamGrants,
        server::handlers::handle_oidc::get_req_session,
    },
    flowcontrol::shed,
    http::HeaderMap,
    htwrap::htserve::{
        self,
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    shared::interface::iam::UserIdentityId,
};

pub enum Identity {
    Token(IamGrants),
    User(UserIdentityId),
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
                return Ok(Some(Identity::Public));
            };
            state.log.log(loga::DEBUG, format!("Request user identified as [{}]", user.0));
            return Ok(Some(Identity::User(user)));
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
        Identity::Public => {
            return Ok(false);
        },
    }
}
