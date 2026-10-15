use serde::{Deserialize, Serialize};

pub const MANAGER_ID: &str = "00000000-0000-4000-8000-000000000001";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Bootstrap,
    ListOrders,
    Order {
        id: String,
    },
    Diagnostics,
    Reports {
        from_date: String,
        to_date: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        include_portfolio: Option<bool>,
    },
    OrderPage {
        search: String,
        page: i64,
        page_size: i64,
        scope: OrderScope,
    },
    ClientInsights,
    Salaries {
        month: String,
    },
    SeparateCosts,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderScope {
    All,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub request_id: String,
    pub actor_id: String,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    ReverseEntry {
        entry_type: CorrectionKind,
        id: String,
        reason: String,
        business_date: String,
    },
    AddSeparateCost {
        category: SeparateCostCategory,
        amount_fils: String,
        description: String,
        business_date: String,
    },
    SetSalaryDivisor {
        month: String,
        expected_version: i64,
        divisor: i64,
    },
    SaveSalary {
        id: Option<String>,
        expected_version: Option<i64>,
        month: String,
        expected_month_version: i64,
        worker_id: String,
        basic_fils: String,
        normal_days_scaled: String,
        hours_per_day_scaled: String,
        fridays: i64,
        overtime_hours_scaled: String,
        notes: String,
    },
    ArchiveSalary {
        id: String,
        expected_version: i64,
        expected_month_version: i64,
        archived: bool,
    },
    SetPaymentDueDate {
        order_id: String,
        expected_version: i64,
        payment_due_date: Option<String>,
    },
    AddOutsideService {
        order_id: String,
        expected_version: i64,
        description: String,
        supplier_text: String,
        charge_fils: String,
        supplier_cost_fils: String,
        business_date: String,
        reference: String,
    },
    ReceivePayment {
        order_id: String,
        amount_fils: String,
        business_date: String,
        method: String,
        reference: String,
        notes: String,
    },
    SaveClient {
        id: Option<String>,
        expected_version: Option<i64>,
        name: String,
        contact_name: String,
        email: String,
        phone: String,
        address: String,
        archived: bool,
    },
    SaveWorker {
        id: Option<String>,
        expected_version: Option<i64>,
        name: String,
        role: String,
        active: bool,
    },
    SaveTemplate {
        id: Option<String>,
        expected_version: Option<i64>,
        name: String,
        default_notes: String,
        archived: bool,
    },
    ReorderTemplates {
        ids: Vec<String>,
        versions: Vec<i64>,
    },
    DeleteTemplate {
        id: String,
        expected_version: i64,
    },
    CreateOrder {
        client_id: String,
        name: String,
        due_date: String,
        notes: String,
        pricing_mode: PricingMode,
        fixed_total_fils: Option<String>,
        items: Vec<NewItem>,
        template_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stage_prices: Option<Vec<StagePriceInput>>,
    },
    UpdateStage {
        id: String,
        expected_version: i64,
        state: StageState,
        notes: String,
        business_date: String,
        reason: Option<String>,
    },
    PostStageEntry {
        order_id: String,
        stage_id: String,
        business_date: String,
        work: Option<WorkInput>,
        expenses: Vec<ExpenseInput>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeparateCostCategory {
    Machine,
    Finish,
    Other,
}
impl SeparateCostCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Machine => "machine",
            Self::Finish => "finish",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PricingMode {
    Unpriced,
    Fixed,
    PerBox,
}
impl PricingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unpriced => "unpriced",
            Self::Fixed => "fixed",
            Self::PerBox => "per_box",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    NotStarted,
    InProgress,
    Completed,
}
impl StageState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewItem {
    pub description: String,
    pub dimensions_text: String,
    pub ordered_boxes: i64,
    pub unit_price_fils: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StagePricingMode {
    Unpriced,
    Fixed,
    PerBox,
    PerUnit,
}
impl StagePricingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unpriced => "unpriced",
            Self::Fixed => "fixed",
            Self::PerBox => "per_box",
            Self::PerUnit => "per_unit",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagePriceInput {
    pub template_id: String,
    pub pricing_mode: StagePricingMode,
    pub fixed_total_fils: Option<String>,
    pub quantity_scaled: Option<String>,
    pub unit_price_fils: Option<String>,
    pub item_prices_fils: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkInput {
    pub item_id: Option<String>,
    pub description: String,
    pub dimensions_text: String,
    pub input_scaled: Option<String>,
    pub output_scaled: Option<String>,
    pub waste_scaled: Option<String>,
    pub worker_id: Option<String>,
    pub worker_minutes: Option<i64>,
    pub machine_description: String,
    pub machine_minutes: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpenseInput {
    pub item_id: Option<String>,
    pub category: ExpenseCategory,
    pub description: String,
    pub supplier_text: String,
    pub reference: String,
    pub pricing_basis: ExpenseBasis,
    pub quantity_scaled: Option<String>,
    pub unit_cost_fils: Option<String>,
    pub amount_fils: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseCategory {
    Material,
    Outsourced,
    Machine,
    Labor,
    Transport,
    Other,
}
impl ExpenseCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Outsourced => "outsourced",
            Self::Machine => "machine",
            Self::Labor => "labor",
            Self::Transport => "transport",
            Self::Other => "other",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseBasis {
    Unit,
    Total,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    Expense,
    Work,
    Payment,
    Service,
    SeparateCost,
}
