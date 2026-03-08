use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct SessionListParams {
    pub project: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct MessageListParams {
    #[serde(default = "default_message_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct SearchParams {
    pub q: String,
    pub kind: Option<String>,
    pub project: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct DateRangeParams {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct LimitParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/generated/")]
pub struct FileQueryParams {
    pub path: Option<String>,
    pub session: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

fn default_message_limit() -> i64 {
    100
}
