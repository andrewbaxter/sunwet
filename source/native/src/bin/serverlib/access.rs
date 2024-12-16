use {
    super::state::State,
    native::interface::config::Config,
    shared::interface::iam::{
        IamTargetId,
        IamUserGroupId,
    },
    std::collections::{
        HashMap,
        HashSet,
    },
};

pub fn compile_nonadmin_access(config: &Config) -> HashMap<IamUserGroupId, NonAdminAccess> {
    let mut out = HashMap::<IamUserGroupId, NonAdminAccess>::new();
    for rule in &config.access.rules {
        let out_rule = out.entry(rule.user_group_id).or_default();
        if rule.read {
            out_rule.read.insert(rule.target_id);
        }
        if rule.write {
            out_rule.write.insert(rule.target_id);
        }
    }
    return out;
}

pub struct AccessRule {
    pub read: bool,
    pub write: bool,
}

#[derive(Default)]
pub struct NonAdminAccess {
    pub read: HashSet<IamTargetId>,
    pub write: HashSet<IamTargetId>,
}

pub enum Identity {
    Admin,
    NonAdmin(Vec<IamUserGroupId>),
}

impl Identity {
    pub fn can_read(&self, state: &State, target_id: IamTargetId) -> bool {
        match self {
            Identity::Admin => {
                return true;
            },
            Identity::NonAdmin(a) => {
                for g in a {
                    if let Some(rule) = state.access.get(g) {
                        if rule.read.contains(&target_id) {
                            return true;
                        }
                    }
                }
                return false;
            },
        }
    }

    pub fn can_write(&self, state: &State, target_id: IamTargetId) -> bool {
        match self {
            Identity::Admin => {
                return true;
            },
            Identity::NonAdmin(a) => {
                for g in a {
                    if let Some(rule) = state.access.get(g) {
                        if rule.write.contains(&target_id) {
                            return true;
                        }
                    }
                }
                return false;
            },
        }
    }
}
