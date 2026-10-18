use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Snapshot,
    Order { id: String },
    History { order_id: String },
    Backups,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub request_id: String,
    pub action: Action,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    SaveClient {
        id: Option<String>,
        expected_version: Option<i64>,
        name: String,
        contact_name: String,
        phone: String,
        email: String,
        address: String,
        notes: String,
        archived: bool,
    },
    SaveTemplate {
        id: Option<String>,
        expected_version: Option<i64>,
        name: String,
        notes: String,
        archived: bool,
    },
    ReorderTemplates {
        ids: Vec<String>,
        versions: Vec<i64>,
    },
    SaveOrder {
        id: Option<String>,
        expected_version: Option<i64>,
        input: OrderInput,
        reason: String,
        deposit: Option<ReceiptInput>,
    },
    ArchiveOrder {
        id: String,
        expected_version: i64,
        archived: bool,
    },
    Receive {
        order_id: String,
        receipt: ReceiptInput,
    },
    CorrectReceipt {
        id: String,
        reason: String,
        business_date: String,
    },
    Profile {
        name: String,
    },
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrderInput {
    pub client_id: String,
    pub name: String,
    pub due_date: Option<String>,
    pub payment_due_date: Option<String>,
    pub notes: String,
    pub status: OrderStatus,
    pub items: Vec<BoxInput>,
    pub stages: Vec<StageInput>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Draft,
    Confirmed,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoxInput {
    pub id: String,
    pub description: String,
    pub dimensions: String,
    pub quantity: i64,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PricingMode {
    Unpriced,
    Fixed,
    PerBox,
    PerUnit,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemPrice {
    pub item_id: String,
    pub unit_price_fils: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StageInput {
    pub template_id: String,
    pub pricing_mode: PricingMode,
    pub fixed_total_fils: Option<String>,
    pub quantity_scaled: Option<String>,
    pub unit_price_fils: Option<String>,
    pub item_prices: Vec<ItemPrice>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInput {
    pub amount_fils: String,
    pub business_date: String,
    pub method: Method,
    pub reference: String,
    pub notes: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Cash,
    Bank,
    Card,
    Other,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDocument {
    pub input: OrderInput,
    pub stage_names: Vec<String>,
    pub stage_totals: Vec<Option<String>>,
}
