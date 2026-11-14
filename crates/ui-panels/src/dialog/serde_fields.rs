//! The serde round trip a dialog's `json_fields` and `set_json_field` share.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The value as a JSON object; `None` when it does not serialise to one.
pub fn serde_fields<T: Serialize>(value: &T) -> Option<serde_json::Value> {
    let _ = value;
    todo!("dl.K3")
}

/// Replaces the top-level key `field` of `value` with `json`, by serialising the value, replacing the key and reading
/// it back. An unknown key or a value `T` cannot hold gives the refusal's message and leaves `value` unchanged.
pub fn set_serde_field<T: Serialize + DeserializeOwned>(value: &mut T, field: &str, json: serde_json::Value) -> Result<(), String> {
    let _ = (value, field, json);
    todo!("dl.K3")
}
