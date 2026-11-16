//! Per-entity display property overrides (BYLAYER when None).

/// Optional overrides for color / linetype / lineweight / draw order.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct EntityProps {
    /// RGB override; `None` = BYLAYER.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    /// Linetype override; `None` = BYLAYER.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linetype: Option<String>,
    /// Lineweight override (hundredths mm); `None` = BYLAYER.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineweight: Option<u16>,
    /// Relative draw order (higher = in front).
    #[serde(default)]
    pub draw_order: i64,
}
