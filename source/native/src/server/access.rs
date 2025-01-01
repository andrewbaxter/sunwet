use {
    super::{
        handlers::handle_oidc,
        state::{
            get_global_config,
            get_user_config,
            State,
        },
    },
    crate::interface::config::IamGrants,
    http::HeaderMap,
    htwrap::htserve::{
        self,
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    shared::interface::iam::{
        UserIdentityId,
    },
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
            return Ok(Some(Identity::Admin));
        }
    }
    if let Some(oidc_state) = &state.oidc_state {
        if let Some(user) = handle_oidc::get_req_identity(&state.log, oidc_state, headers).await {
            return Ok(Some(Identity::User(user)));
        }
    }
    if !global_config.config.public_iam_grants.is_empty() {
        return Ok(Some(Identity::Public));
    }
    return Ok(None);
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
