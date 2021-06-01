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
#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "filter_type", content = "with_value")]
pub enum FilterType {
    All,
    Category(String),
    CreationDate(DateTime<Utc>),
}

/// How the data should be ordered when requesting the
/// index list.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderBy {
    CreationDate,
    TotalSize,
}

/// A result when listing all items in the server.
#[derive(Serialize)]
pub struct IndexResult {
    file_id: Uuid,
    total_size: usize,
    created_on: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct FilesListPayload {
    pub filter: FilterType,
    pub order: OrderBy,
    pub page: Option<usize>,
}
