use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct SessionListParams {
    pub project: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub outcome: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for SessionListParams {
    fn default() -> Self {
        Self {
            project: None,
            from: None,
            to: None,
            outcome: None,
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct MessageListParams {
    pub limit: i64,
    pub offset: i64,
}

impl Default for MessageListParams {
    fn default() -> Self {
        Self {
            limit: default_message_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct SearchParams {
    pub q: String,
    pub kind: Option<String>,
    pub project: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            q: String::new(),
            kind: None,
            project: None,
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, TS, Default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct DateRangeParams {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct LimitParams {
    pub limit: i64,
    pub offset: i64,
}

impl Default for LimitParams {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[serde(default)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct FileQueryParams {
    pub path: Option<String>,
    pub session: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for FileQueryParams {
    fn default() -> Self {
        Self {
            path: None,
            session: None,
            limit: default_limit(),
            offset: 0,
        }
    }
}

fn default_limit() -> i64 {
    20
}

fn default_message_limit() -> i64 {
    100
}
