use {
    super::{
        handlers::handle_oidc,
        state::{
            get_global_config,
            get_user_config,
            State,
        },
    },
    crate::interface::config::UserAccess,
    flowcontrol::exenum,
    http::HeaderMap,
    htwrap::htserve::{
        self,
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    shared::interface::iam::{
        IamTargetId,
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
    if global_config.config.public_access.is_some() {
        return Ok(Some(Identity::Public));
    }
    return Ok(None);
}

pub async fn can_write(state: &State, identity: &Identity) -> Result<bool, loga::Error> {
    match identity {
        Identity::Admin => {
            return Ok(true);
        },
        Identity::User(u) => {
            let user_config = get_user_config(&state, u).await?;
            if exenum!(&user_config.access, UserAccess:: ReadWrite =>()).is_some() {
                return Ok(true);
            } else {
                return Ok(false);
            }
        },
        Identity::Public => {
            return Ok(false);
        },
    }
}

pub enum CanRead {
    All,
    Restricted(Vec<IamTargetId>),
    No,
}

pub async fn can_read(state: &State, identity: &Identity) -> Result<CanRead, loga::Error> {
    match identity {
        Identity::Admin => {
            return Ok(CanRead::All);
        },
        Identity::User(u) => {
            let user_config = get_user_config(&state, u).await?;
            match &user_config.access {
                UserAccess::ReadWrite => {
                    return Ok(CanRead::All);
                },
                UserAccess::Read(targets) => {
                    return Ok(CanRead::Restricted(targets.clone()));
                },
            }
        },
        Identity::Public => {
            let global_config = get_global_config(&state).await?;
            if let Some(targets) = &global_config.config.public_access {
                return Ok(CanRead::Restricted(targets.clone()));
            } else {
                return Ok(CanRead::No);
            }
        },
    }
}

pub enum ReadRestriction {
    None,
    Some(Vec<IamTargetId>),
}
