use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use serde::{Deserialize, Serialize};


/// A set of filters that can be used to view
/// entities via the REST API on the admin panel.
///
/// Example:
///
/// ```json
/// {
///     "filter": {
///         "filter_type": "category",
///         "with_value": "cats",
///     }
/// }
/// ```
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "filter_type", content = "with_value")]
pub enum FilterType {
    All,
    Category(String),
    CreationDate(DateTime<Utc>),
}

/// How the data should be ordered when requesting the
/// index list.
#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderBy {
    CreationDate,
    TotalSize,
}

impl OrderBy {
    pub fn as_str(&self) -> &str {
        match self {
            OrderBy::CreationDate => {
                "insert_date"
            },
            OrderBy::TotalSize => {
                "total_size"
            },
        }
    }
}

/// A result when listing all items in the server.
#[derive(Serialize)]
pub struct IndexResult {
    pub file_id: Uuid,
    pub category: String,
    pub total_size: i64,
    pub created_on: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct FilesListPayload {
    pub filter: Option<FilterType>,
    pub order: Option<OrderBy>,
    pub page: Option<usize>,
}


