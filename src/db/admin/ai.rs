//! AI provider catalog + usage + budget.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::{Db, record_id_key_to_string};
use crate::error::Error;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
#[surreal(crate = "surrealdb::types")]
pub struct AiModel {
    pub id: Option<RecordId>,
    pub provider: String,
    pub name: String,
    pub label: String,
    pub note: Option<String>,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct AiModelResponse {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub label: String,
    pub note: Option<String>,
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub is_active: bool,
}

impl From<AiModel> for AiModelResponse {
    fn from(m: AiModel) -> Self {
        Self {
            id: m
                .id
                .map(|r| record_id_key_to_string(&r.key))
                .unwrap_or_default(),
            provider: m.provider,
            name: m.name,
            label: m.label,
            note: m.note,
            input_cost_per_million: m.input_cost_per_million,
            output_cost_per_million: m.output_cost_per_million,
            is_active: m.is_active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UsageOverview {
    pub period: String,
    pub tokens_used: i64,
    pub est_cost_usd: f64,
    pub monthly_budget_usd: f64,
    pub budget_used_pct: f64,
    pub by_model: Vec<UsageByModel>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct UsageByModel {
    pub model_id: String,
    pub model_label: String,
    pub tokens_used: i64,
    pub cost_usd: f64,
}

pub struct AiRepo {
    pub db: Db,
}

impl AiRepo {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn list_models(&self) -> Result<Vec<AiModelResponse>, Error> {
        let mut resp = self
            .db
            .query("SELECT * FROM ai_model WHERE is_active = true ORDER BY provider, name")
            .await?;
        let rows: Vec<AiModel> = resp.take(0)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Aggregate usage for the current month (simple WHERE on `occurred_at`).
    pub async fn usage_overview(&self, period: &str) -> Result<UsageOverview, Error> {
        let cutoff = match period {
            "day" => "time::now() - 1d",
            "week" => "time::now() - 7d",
            _ => "time::now() - 30d",
        };

        // Total tokens + cost.
        let sql = format!(
            "RETURN {{ \
               total_tokens: (SELECT math::sum(prompt_tokens + completion_tokens) AS s FROM token_usage WHERE occurred_at > {0} GROUP ALL)[0].s ?? 0, \
               total_cost: (SELECT math::sum(cost_usd) AS s FROM token_usage WHERE occurred_at > {0} GROUP ALL)[0].s ?? 0.0, \
               budget: (SELECT VALUE monthly_budget_usd FROM org_setting:singleton)[0] ?? 100.0 \
             }}",
            cutoff
        );
        let mut resp = self.db.query(sql).await?;

        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct Totals {
            total_tokens: i64,
            total_cost: f64,
            budget: f64,
        }
        let totals_opt: Option<Totals> = resp.take(0)?;
        let t: Totals = totals_opt.unwrap_or(Totals {
            total_tokens: 0,
            total_cost: 0.0,
            budget: 100.0,
        });

        // Per-model breakdown.
        let breakdown_sql = format!(
            "SELECT model.id AS mid, model.label AS label, \
                    math::sum(prompt_tokens + completion_tokens) AS tokens, \
                    math::sum(cost_usd) AS cost \
             FROM token_usage WHERE occurred_at > {0} GROUP BY mid, label",
            cutoff
        );
        let mut bresp = self.db.query(breakdown_sql).await?;

        #[derive(Deserialize, SurrealValue)]
        #[surreal(crate = "surrealdb::types")]
        struct B {
            mid: Option<RecordId>,
            label: Option<String>,
            tokens: Option<i64>,
            cost: Option<f64>,
        }
        let rows: Vec<B> = bresp.take(0)?;
        let by_model = rows
            .into_iter()
            .filter_map(|r| {
                let mid = r.mid.map(|rid| record_id_key_to_string(&rid.key))?;
                Some(UsageByModel {
                    model_id: mid,
                    model_label: r.label.unwrap_or_default(),
                    tokens_used: r.tokens.unwrap_or(0),
                    cost_usd: r.cost.unwrap_or(0.0),
                })
            })
            .collect();

        let pct = if t.budget > 0.0 {
            (t.total_cost / t.budget) * 100.0
        } else {
            0.0
        };

        Ok(UsageOverview {
            period: period.to_string(),
            tokens_used: t.total_tokens,
            est_cost_usd: t.total_cost,
            monthly_budget_usd: t.budget,
            budget_used_pct: pct,
            by_model,
        })
    }

    /// Append a token-usage row. Called by the pipeline worker after each
    /// AI invocation.
    pub async fn record_usage(
        &self,
        user_id: &str,
        job_id: Option<&str>,
        model_id: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
        cost_usd: f64,
    ) -> Result<(), Error> {
        self.db
            .query(
                "CREATE token_usage SET \
                   user = type::record('user', $uid), \
                   job  = (IF $jid = NONE THEN NONE ELSE type::record('ingestion_job', $jid) END), \
                   model = type::record('ai_model', $mid), \
                   prompt_tokens = $pt, completion_tokens = $ct, cost_usd = $cost",
            )
            .bind(("uid", user_id.to_string()))
            .bind(("jid", job_id.map(str::to_string)))
            .bind(("mid", model_id.to_string()))
            .bind(("pt", prompt_tokens))
            .bind(("ct", completion_tokens))
            .bind(("cost", cost_usd))
            .await?;
        Ok(())
    }
}
