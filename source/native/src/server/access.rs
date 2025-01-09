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
    Admin,
    User(UserIdentityId),
    Public,
}

// None = can't be identified = 401.
pub async fn identify_requester(
    state: &State,
    headers: &HeaderMap,
) -> Result<Option<Identity>, VisErr<loga::Error>> {
    let global_config = get_global_config(state).await.err_internal()?;
    if let Some(want_token) = global_config.admin_token.as_ref() {
        if let Ok(got_token) = htserve::auth::get_auth_token(headers) {
            if !htserve::auth::check_auth_token_hash(&want_token, &got_token) {
                return Ok(None);
            }
            state.log.log(loga::DEBUG, "Request user identified as admin");
            return Ok(Some(Identity::Admin));
        }
        state.log.log(loga::DEBUG, "Request user has no admin token");
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
                return Ok(None);
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
        Identity::Admin => {
            return Ok(true);
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
