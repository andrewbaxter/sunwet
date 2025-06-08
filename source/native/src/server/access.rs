use {
    super::state::{
        get_global_config,
        get_user_config,
        State,
    },
    crate::{
        interface::{
            config::IamGrants,
            triple::DbFileHash,
        },
        server::{
            db,
            dbutil::tx,
            state::get_iam_grants,
            subsystems::oidc::get_req_session,
        },
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
    loga::{
        ea,
        DebugDisplay,
    },
    shared::interface::{
        iam::UserIdentityId,
        triple::FileHash,
        wire::link::COOKIE_LINK_SESSION,
    },
    std::collections::HashSet,
};

#[derive(Debug)]
pub enum Identity {
    Token(IamGrants),
    User(UserIdentityId),
    Link(String),
    Public,
}

pub async fn identify_requester(state: &State, headers: &HeaderMap) -> Result<Identity, VisErr<loga::Error>> {
    let global_config = get_global_config(state).await.err_internal()?;
    if let Ok(got_token) = htserve::auth::get_auth_token(headers) {
        if let Some(grants) = global_config.api_tokens.get(&got_token) {
            return Ok(Identity::Token(grants.clone()));
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
            return Ok(Identity::User(user));
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
            return Ok(Identity::Link(c.value().to_string()));
        }
    }
    return Ok(Identity::Public);
}

#[derive(Debug)]
pub enum AccessRes {
    Yes,
    NoIdent,
    NoAccess,
}

pub async fn check_is_admin(state: &State, identity: &Identity, context: &str) -> Result<AccessRes, loga::Error> {
    let out;
    match identity {
        Identity::Token(grants) => {
            match grants {
                IamGrants::Admin => {
                    out = AccessRes::Yes;
                },
                _ => {
                    out = AccessRes::NoAccess;
                },
            }
        },
        Identity::User(u) => {
            let user_config = get_user_config(&state, u).await?;
            match &user_config.iam_grants {
                IamGrants::Admin => {
                    out = AccessRes::Yes;
                },
                _ => {
                    out = AccessRes::NoAccess;
                },
            }
        },
        Identity::Link(_) => {
            out = AccessRes::NoAccess;
        },
        Identity::Public => {
            out = AccessRes::NoIdent;
        },
    };
    state
        .log
        .log_with(
            loga::DEBUG,
            format!("Admin access result for context: {}", context),
            ea!(identity = identity.dbg_str(), result = out.dbg_str()),
        );
    return Ok(out);
}

pub async fn can_access_file(state: &State, identity: &Identity, file: &FileHash) -> Result<AccessRes, loga::Error> {
    let grants = get_iam_grants(state, identity).await?;
    let out = shed!{
        'done _;
        match &grants {
            IamGrants::Admin => {
                break 'done AccessRes::Yes;
            },
            IamGrants::Limited(grants) => {
                let stored_access = tx(&state.db, {
                    let file = DbFileHash(file.clone());
                    move |txn| Ok(db::file_access_get(txn, &file)?)
                }).await?.into_iter().collect::<HashSet<_>>();
                for grant in &grants.forms {
                    if stored_access.contains(grant) {
                        break 'done AccessRes::Yes;
                    }
                }
            },
        }
        match identity {
            Identity::Token(_) => { },
            Identity::User(_) => { },
            Identity::Link(l) => {
                if let Some(session) = state.link_sessions.get(l).await {
                    if session.public_files.lock().unwrap().contains(&file) {
                        break 'done AccessRes::Yes;
                    }
                }
            },
            Identity::Public => {
                break 'done AccessRes::NoIdent;
            },
        }
        break 'done AccessRes::NoAccess;
    };
    state
        .log
        .log_with(
            loga::DEBUG,
            "File access result",
            ea!(
                identity = identity.dbg_str(),
                file = file.dbg_str(),
                grants = grants.dbg_str(),
                result = out.dbg_str()
            ),
        );
    return Ok(out);
}
