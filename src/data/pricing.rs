use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModelPricing {
    pub name: &'static str,
    pub display_name: &'static str,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_write_per_mtok: Option<f64>,
    pub cache_read_per_mtok: Option<f64>,
}

const MODEL_PRICING: &[(&str, ModelPricing)] = &[
    (
        "claude-opus-4-5-20251101",
        ModelPricing {
            name: "claude-opus-4-5-20251101",
            display_name: "Opus 4.5",
            input_per_mtok: 5.0,
            output_per_mtok: 25.0,
            cache_write_per_mtok: Some(6.25),
            cache_read_per_mtok: Some(0.5),
        },
    ),
    (
        "claude-sonnet-4-5-20251101",
        ModelPricing {
            name: "claude-sonnet-4-5-20251101",
            display_name: "Sonnet 4.5",
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: Some(3.75),
            cache_read_per_mtok: Some(0.3),
        },
    ),
    (
        "claude-haiku-4-5-20251001",
        ModelPricing {
            name: "claude-haiku-4-5-20251001",
            display_name: "Haiku 4.5",
            input_per_mtok: 1.0,
            output_per_mtok: 5.0,
            cache_write_per_mtok: Some(1.25),
            cache_read_per_mtok: Some(0.1),
        },
    ),
    (
        "claude-opus-4-20250514",
        ModelPricing {
            name: "claude-opus-4-20250514",
            display_name: "Opus 4",
            input_per_mtok: 15.0,
            output_per_mtok: 75.0,
            cache_write_per_mtok: None,
            cache_read_per_mtok: None,
        },
    ),
    (
        "claude-sonnet-4-20250514",
        ModelPricing {
            name: "claude-sonnet-4-20250514",
            display_name: "Sonnet 4",
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: None,
            cache_read_per_mtok: None,
        },
    ),
    (
        "claude-3-5-sonnet-20241022",
        ModelPricing {
            name: "claude-3-5-sonnet-20241022",
            display_name: "Sonnet 3.5",
            input_per_mtok: 3.0,
            output_per_mtok: 15.0,
            cache_write_per_mtok: None,
            cache_read_per_mtok: None,
        },
    ),
    (
        "claude-3-haiku-20240307",
        ModelPricing {
            name: "claude-3-haiku-20240307",
            display_name: "Haiku 3",
            input_per_mtok: 0.25,
            output_per_mtok: 1.25,
            cache_write_per_mtok: None,
            cache_read_per_mtok: None,
        },
    ),
];

const DEFAULT_PRICING: ModelPricing = ModelPricing {
    name: "unknown",
    display_name: "Unknown Model",
    input_per_mtok: 3.0,
    output_per_mtok: 15.0,
    cache_write_per_mtok: None,
    cache_read_per_mtok: None,
};

pub fn get_model_pricing(model_name: Option<&str>) -> ModelPricing {
    let name = match model_name {
        Some(n) => n,
        None => return DEFAULT_PRICING,
    };

    // Exact match
    for (key, pricing) in MODEL_PRICING {
        if *key == name {
            return pricing.clone();
        }
    }

    // Partial match: compare first 3 dash-separated segments
    let name_lower = name.to_lowercase();
    let name_prefix = first_n_segments(&name_lower, 3);
    for (key, pricing) in MODEL_PRICING {
        let key_prefix = first_n_segments(key, 3);
        if name_prefix == key_prefix {
            return pricing.clone();
        }
    }

    // Family match
    if name_lower.contains("opus-4-5") || name_lower.contains("opus-4.5") {
        return MODEL_PRICING[0].1.clone(); // Opus 4.5
    }
    if name_lower.contains("sonnet-4-5") || name_lower.contains("sonnet-4.5") {
        return MODEL_PRICING[1].1.clone(); // Sonnet 4.5
    }
    if name_lower.contains("haiku-4-5") || name_lower.contains("haiku-4.5") {
        return MODEL_PRICING[2].1.clone(); // Haiku 4.5
    }
    if name_lower.contains("opus") {
        return MODEL_PRICING[3].1.clone(); // Opus 4
    }
    if name_lower.contains("sonnet") {
        return MODEL_PRICING[4].1.clone(); // Sonnet 4
    }
    if name_lower.contains("haiku") {
        return MODEL_PRICING[6].1.clone(); // Haiku 3
    }

    DEFAULT_PRICING
}

fn first_n_segments(s: &str, n: usize) -> String {
    s.split('-').take(n).collect::<Vec<_>>().join("-")
}

pub fn calculate_cost(model_name: Option<&str>, input_tokens: i64, output_tokens: i64) -> f64 {
    let pricing = get_model_pricing(model_name);
    let input_cost = (input_tokens as f64 / 1_000_000.0) * pricing.input_per_mtok;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * pricing.output_per_mtok;
    input_cost + output_cost
}

pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${:.4}", cost)
    } else if cost < 1.0 {
        format!("${:.3}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

#[allow(dead_code)]
pub fn get_all_pricing() -> Vec<ModelPricing> {
    MODEL_PRICING.iter().map(|(_, p)| p.clone()).collect()
}

// --- Subscription & Billing ---

fn get_subscription_prices() -> &'static [(&'static str, f64)] {
    &[
        ("free", 0.0),
        ("pro", 20.0),
        ("max5x", 100.0),
        ("max20x", 200.0),
    ]
}

pub fn get_subscription_price(tier: &str) -> f64 {
    let normalized = tier.to_lowercase().replace(['-', ' '], "");
    for (key, price) in get_subscription_prices() {
        if *key == normalized {
            return *price;
        }
    }
    200.0
}

#[derive(Debug, Clone, Serialize)]
pub struct BillingCycle {
    pub start_ms: i64,
    pub end_ms: i64,
    pub billing_day: u32,
    pub total_days: i64,
    pub days_elapsed: i64,
    pub days_remaining: i64,
}

pub fn calculate_billing_cycle(billing_day: u32, now_ms: i64) -> BillingCycle {
    use chrono::{Datelike, NaiveDate, TimeZone, Utc};

    let now = Utc.timestamp_millis_opt(now_ms).unwrap();
    let today = now.date_naive();

    let (start_date, end_date) = if today.day() < billing_day {
        // Cycle started last month
        let start = prev_month_date(today, billing_day);
        let end = NaiveDate::from_ymd_opt(today.year(), today.month(), billing_day)
            .unwrap_or_else(|| last_day_of_month(today.year(), today.month()));
        (start, end)
    } else {
        // Cycle started this month
        let start = NaiveDate::from_ymd_opt(today.year(), today.month(), billing_day)
            .unwrap_or_else(|| last_day_of_month(today.year(), today.month()));
        let end = next_month_date(today, billing_day);
        (start, end)
    };

    let start_ms = start_date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    let end_ms = end_date
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();

    let total_days = (end_ms - start_ms) / 86_400_000;
    let days_elapsed = (now_ms - start_ms) / 86_400_000;
    let days_remaining = total_days - days_elapsed;

    BillingCycle {
        start_ms,
        end_ms,
        billing_day,
        total_days,
        days_elapsed,
        days_remaining,
    }
}

fn prev_month_date(today: chrono::NaiveDate, day: u32) -> chrono::NaiveDate {
    use chrono::{Datelike, NaiveDate};
    let (year, month) = if today.month() == 1 {
        (today.year() - 1, 12)
    } else {
        (today.year(), today.month() - 1)
    };
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| last_day_of_month(year, month))
}

fn next_month_date(today: chrono::NaiveDate, day: u32) -> chrono::NaiveDate {
    use chrono::{Datelike, NaiveDate};
    let (year, month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| last_day_of_month(year, month))
}

fn last_day_of_month(year: i32, month: u32) -> chrono::NaiveDate {
    use chrono::NaiveDate;
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
}

// --- Break-even analysis ---

#[derive(Debug, Clone, Serialize)]
pub struct BreakEvenAnalysis {
    pub price: f64,
    pub break_even_output_tokens: i64,
    pub break_even_input_tokens: i64,
}

pub fn calculate_break_even(subscription_price: f64) -> BreakEvenAnalysis {
    // Uses Opus 4.5 pricing: $5/MTok input, $25/MTok output
    let break_even_output_tokens = ((subscription_price / 25.0) * 1_000_000.0).round() as i64;
    let break_even_input_tokens = ((subscription_price / 5.0) * 1_000_000.0).round() as i64;
    BreakEvenAnalysis {
        price: subscription_price,
        break_even_output_tokens,
        break_even_input_tokens,
    }
}

#[allow(dead_code)]
pub fn get_max_tier_break_even() -> (BreakEvenAnalysis, BreakEvenAnalysis) {
    (calculate_break_even(100.0), calculate_break_even(200.0))
}

pub fn project_cycle_cost(current_cost: f64, days_elapsed: i64, total_days: i64) -> f64 {
    if days_elapsed <= 0 {
        return 0.0;
    }
    (current_cost / days_elapsed as f64) * total_days as f64
}

// --- Competitor pricing ---

#[derive(Debug, Clone, Serialize)]
pub struct CompetitorPricing {
    pub name: &'static str,
    pub url: &'static str,
    pub plans: Vec<CompetitorPlan>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompetitorPlan {
    pub name: &'static str,
    pub price: f64,
    pub plan_type: &'static str,
    pub limits: Option<&'static str>,
}

pub fn get_competitor_pricing() -> Vec<CompetitorPricing> {
    vec![
        CompetitorPricing {
            name: "Claude (Anthropic)",
            url: "https://claude.com/pricing",
            plans: vec![
                CompetitorPlan {
                    name: "Free",
                    price: 0.0,
                    plan_type: "individual",
                    limits: Some("Basic usage"),
                },
                CompetitorPlan {
                    name: "Pro",
                    price: 20.0,
                    plan_type: "individual",
                    limits: Some("More usage, Claude Code access"),
                },
                CompetitorPlan {
                    name: "Max 5x",
                    price: 100.0,
                    plan_type: "individual",
                    limits: Some("5x Pro usage"),
                },
                CompetitorPlan {
                    name: "Max 20x",
                    price: 200.0,
                    plan_type: "individual",
                    limits: Some("20x Pro usage"),
                },
                CompetitorPlan {
                    name: "Team Standard",
                    price: 30.0,
                    plan_type: "team",
                    limits: Some("Per seat, min 5 members"),
                },
                CompetitorPlan {
                    name: "Team Premium",
                    price: 150.0,
                    plan_type: "team",
                    limits: Some("Per seat, includes Claude Code"),
                },
            ],
        },
        CompetitorPricing {
            name: "GitHub Copilot",
            url: "https://github.com/features/copilot",
            plans: vec![
                CompetitorPlan {
                    name: "Free",
                    price: 0.0,
                    plan_type: "individual",
                    limits: Some("2,000 completions/mo, 50 chat/mo"),
                },
                CompetitorPlan {
                    name: "Pro",
                    price: 10.0,
                    plan_type: "individual",
                    limits: Some("Unlimited completions, 300 premium req/mo"),
                },
                CompetitorPlan {
                    name: "Pro+",
                    price: 39.0,
                    plan_type: "individual",
                    limits: Some("1,500 premium req/mo"),
                },
                CompetitorPlan {
                    name: "Business",
                    price: 19.0,
                    plan_type: "team",
                    limits: Some("300 premium req/mo per user"),
                },
                CompetitorPlan {
                    name: "Enterprise",
                    price: 39.0,
                    plan_type: "enterprise",
                    limits: Some("1,000 premium req/mo per user"),
                },
            ],
        },
        CompetitorPricing {
            name: "Cursor",
            url: "https://cursor.com/pricing",
            plans: vec![
                CompetitorPlan {
                    name: "Hobby",
                    price: 0.0,
                    plan_type: "individual",
                    limits: Some("Limited Agent & Tab completions"),
                },
                CompetitorPlan {
                    name: "Pro",
                    price: 20.0,
                    plan_type: "individual",
                    limits: Some("Extended Agent, unlimited Tabs"),
                },
                CompetitorPlan {
                    name: "Pro+",
                    price: 60.0,
                    plan_type: "individual",
                    limits: Some("3x usage on all models"),
                },
                CompetitorPlan {
                    name: "Ultra",
                    price: 200.0,
                    plan_type: "individual",
                    limits: Some("20x usage, priority features"),
                },
                CompetitorPlan {
                    name: "Teams",
                    price: 40.0,
                    plan_type: "team",
                    limits: Some("Shared chats, SSO, RBAC"),
                },
            ],
        },
        CompetitorPricing {
            name: "Windsurf",
            url: "https://windsurf.com/pricing",
            plans: vec![
                CompetitorPlan {
                    name: "Free",
                    price: 0.0,
                    plan_type: "individual",
                    limits: Some("25 prompt credits/mo"),
                },
                CompetitorPlan {
                    name: "Pro",
                    price: 15.0,
                    plan_type: "individual",
                    limits: Some("500 credits/mo"),
                },
                CompetitorPlan {
                    name: "Teams",
                    price: 30.0,
                    plan_type: "team",
                    limits: Some("500 credits/user/mo"),
                },
            ],
        },
        CompetitorPricing {
            name: "Tabnine",
            url: "https://tabnine.com/pricing",
            plans: vec![CompetitorPlan {
                name: "Agentic Platform",
                price: 59.0,
                plan_type: "individual",
                limits: Some("Unlimited with own LLM"),
            }],
        },
        CompetitorPricing {
            name: "Amazon Q Developer",
            url: "https://aws.amazon.com/q/developer/pricing/",
            plans: vec![
                CompetitorPlan {
                    name: "Free",
                    price: 0.0,
                    plan_type: "individual",
                    limits: Some("50 agentic req/mo"),
                },
                CompetitorPlan {
                    name: "Pro",
                    price: 19.0,
                    plan_type: "team",
                    limits: Some("4,000 lines/mo pooled"),
                },
            ],
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct ValueComparison {
    pub service: String,
    pub plan: String,
    pub price: f64,
    pub savings: f64,
    pub savings_percent: f64,
}

pub fn get_value_comparison(api_equivalent_cost: f64) -> Vec<ValueComparison> {
    let mut comparisons = Vec::new();

    for competitor in get_competitor_pricing() {
        for plan in &competitor.plans {
            if plan.plan_type != "individual" || plan.price <= 0.0 {
                continue;
            }
            let savings = api_equivalent_cost - plan.price;
            let savings_percent = if api_equivalent_cost > 0.0 {
                (savings / api_equivalent_cost) * 100.0
            } else {
                0.0
            };
            comparisons.push(ValueComparison {
                service: competitor.name.to_string(),
                plan: plan.name.to_string(),
                price: plan.price,
                savings,
                savings_percent,
            });
        }
    }

    comparisons.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    comparisons
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn exact_match_opus_45() {
        let p = get_model_pricing(Some("claude-opus-4-5-20251101"));
        assert_eq!(p.display_name, "Opus 4.5");
        assert_eq!(p.input_per_mtok, 5.0);
        assert_eq!(p.output_per_mtok, 25.0);
    }

    #[test]
    fn exact_match_sonnet_45() {
        let p = get_model_pricing(Some("claude-sonnet-4-5-20251101"));
        assert_eq!(p.display_name, "Sonnet 4.5");
    }

    #[test]
    fn exact_match_haiku_45() {
        let p = get_model_pricing(Some("claude-haiku-4-5-20251001"));
        assert_eq!(p.display_name, "Haiku 4.5");
    }

    #[test]
    fn exact_match_opus_4() {
        let p = get_model_pricing(Some("claude-opus-4-20250514"));
        assert_eq!(p.display_name, "Opus 4");
        assert_eq!(p.input_per_mtok, 15.0);
    }

    #[test]
    fn exact_match_sonnet_35() {
        let p = get_model_pricing(Some("claude-3-5-sonnet-20241022"));
        assert_eq!(p.display_name, "Sonnet 3.5");
    }

    #[test]
    fn exact_match_haiku_3() {
        let p = get_model_pricing(Some("claude-3-haiku-20240307"));
        assert_eq!(p.display_name, "Haiku 3");
        assert_eq!(p.input_per_mtok, 0.25);
    }

    #[test]
    fn family_match_opus() {
        let p = get_model_pricing(Some("some-opus-model"));
        assert_eq!(p.display_name, "Opus 4");
    }

    #[test]
    fn family_match_sonnet() {
        let p = get_model_pricing(Some("some-sonnet-model"));
        assert_eq!(p.display_name, "Sonnet 4");
    }

    #[test]
    fn family_match_haiku() {
        let p = get_model_pricing(Some("some-haiku-model"));
        assert_eq!(p.display_name, "Haiku 3");
    }

    #[test]
    fn family_match_opus_45_variant() {
        let p = get_model_pricing(Some("claude-opus-4-5-extended"));
        assert_eq!(p.display_name, "Opus 4.5");
    }

    #[test]
    fn family_match_sonnet_45_variant() {
        let p = get_model_pricing(Some("claude-sonnet-4.5-new"));
        assert_eq!(p.display_name, "Sonnet 4.5");
    }

    #[test]
    fn none_returns_default() {
        let p = get_model_pricing(None);
        assert_eq!(p.display_name, "Unknown Model");
        assert_eq!(p.input_per_mtok, 3.0);
    }

    #[test]
    fn unknown_model_returns_default() {
        let p = get_model_pricing(Some("totally-unknown-model"));
        assert_eq!(p.display_name, "Unknown Model");
    }

    #[test]
    fn calculate_cost_sonnet() {
        let cost = calculate_cost(Some("claude-sonnet-4-5-20251101"), 1_000_000, 1_000_000);
        // $3/MTok input + $15/MTok output = $18
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let cost = calculate_cost(Some("claude-opus-4-5-20251101"), 0, 0);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn calculate_cost_unknown_model() {
        let cost = calculate_cost(None, 1_000_000, 1_000_000);
        // Default: $3 + $15 = $18
        assert!((cost - 18.0).abs() < 0.001);
    }

    #[test]
    fn format_cost_small() {
        assert_eq!(format_cost(0.001), "$0.0010");
        assert_eq!(format_cost(0.0001), "$0.0001");
    }

    #[test]
    fn format_cost_medium() {
        assert_eq!(format_cost(0.123), "$0.123");
        assert_eq!(format_cost(0.5), "$0.500");
    }

    #[test]
    fn format_cost_large() {
        assert_eq!(format_cost(1.5), "$1.50");
        assert_eq!(format_cost(100.0), "$100.00");
    }

    #[test]
    fn format_cost_zero() {
        assert_eq!(format_cost(0.0), "$0.0000");
    }

    #[test]
    fn get_all_pricing_returns_all() {
        let all = get_all_pricing();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn subscription_price_known() {
        assert_eq!(get_subscription_price("free"), 0.0);
        assert_eq!(get_subscription_price("pro"), 20.0);
        assert_eq!(get_subscription_price("max5x"), 100.0);
        assert_eq!(get_subscription_price("max20x"), 200.0);
    }

    #[test]
    fn subscription_price_normalized() {
        assert_eq!(get_subscription_price("Max-5x"), 100.0);
        assert_eq!(get_subscription_price("MAX 20X"), 200.0);
        assert_eq!(get_subscription_price("Pro"), 20.0);
    }

    #[test]
    fn subscription_price_unknown() {
        assert_eq!(get_subscription_price("enterprise"), 200.0);
    }

    #[test]
    fn billing_cycle_mid_month() {
        // Jan 15, billing day 6 -> cycle started Jan 6
        let jan15 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let cycle = calculate_billing_cycle(6, jan15);
        assert_eq!(cycle.billing_day, 6);
        assert!(cycle.days_elapsed > 0);
        assert!(cycle.days_remaining >= 0);
        assert_eq!(cycle.total_days, cycle.days_elapsed + cycle.days_remaining);
    }

    #[test]
    fn billing_cycle_before_billing_day() {
        // Jan 3, billing day 6 -> cycle started Dec 6
        let jan3 = chrono::NaiveDate::from_ymd_opt(2024, 1, 3)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let cycle = calculate_billing_cycle(6, jan3);
        assert!(cycle.total_days >= 28);
        assert!(cycle.days_elapsed > 0);
    }

    #[test]
    fn billing_cycle_on_billing_day() {
        // Jan 6, billing day 6 -> cycle started Jan 6
        let jan6 = chrono::NaiveDate::from_ymd_opt(2024, 1, 6)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let cycle = calculate_billing_cycle(6, jan6);
        assert_eq!(cycle.days_elapsed, 0);
    }

    #[test]
    fn break_even_max5x() {
        let be = calculate_break_even(100.0);
        assert_eq!(be.price, 100.0);
        assert_eq!(be.break_even_output_tokens, 4_000_000);
        assert_eq!(be.break_even_input_tokens, 20_000_000);
    }

    #[test]
    fn break_even_max20x() {
        let be = calculate_break_even(200.0);
        assert_eq!(be.break_even_output_tokens, 8_000_000);
        assert_eq!(be.break_even_input_tokens, 40_000_000);
    }

    #[test]
    fn break_even_zero() {
        let be = calculate_break_even(0.0);
        assert_eq!(be.break_even_output_tokens, 0);
        assert_eq!(be.break_even_input_tokens, 0);
    }

    #[test]
    fn max_tier_break_even() {
        let (max5x, max20x) = get_max_tier_break_even();
        assert_eq!(max5x.price, 100.0);
        assert_eq!(max20x.price, 200.0);
    }

    #[test]
    fn project_cycle_cost_normal() {
        let projected = project_cycle_cost(10.0, 15, 30);
        assert!((projected - 20.0).abs() < 0.001);
    }

    #[test]
    fn project_cycle_cost_zero_elapsed() {
        assert_eq!(project_cycle_cost(10.0, 0, 30), 0.0);
    }

    #[test]
    fn project_cycle_cost_negative_elapsed() {
        assert_eq!(project_cycle_cost(10.0, -1, 30), 0.0);
    }

    #[test]
    fn competitor_pricing_count() {
        let competitors = get_competitor_pricing();
        assert_eq!(competitors.len(), 6);
    }

    #[test]
    fn competitor_pricing_claude_first() {
        let competitors = get_competitor_pricing();
        assert_eq!(competitors[0].name, "Claude (Anthropic)");
    }

    #[test]
    fn value_comparison_positive() {
        let comparisons = get_value_comparison(500.0);
        assert!(!comparisons.is_empty());
        // All individual plans with price > 0
        for c in &comparisons {
            assert!(c.price > 0.0);
        }
        // Sorted by price ascending
        for w in comparisons.windows(2) {
            assert!(w[0].price <= w[1].price);
        }
    }

    #[test]
    fn value_comparison_zero_cost() {
        let comparisons = get_value_comparison(0.0);
        for c in &comparisons {
            assert_eq!(c.savings_percent, 0.0);
        }
    }

    #[test]
    fn first_n_segments_works() {
        assert_eq!(first_n_segments("a-b-c-d", 3), "a-b-c");
        assert_eq!(first_n_segments("a-b", 3), "a-b");
        assert_eq!(first_n_segments("abc", 3), "abc");
    }

    #[test]
    fn partial_match_different_date() {
        // Same model prefix, different date suffix
        let p = get_model_pricing(Some("claude-opus-4-5-20260101"));
        assert_eq!(p.display_name, "Opus 4.5");
    }

    #[test]
    fn last_day_of_february() {
        let d = last_day_of_month(2024, 2); // Leap year
        assert_eq!(d.day(), 29);
        let d2 = last_day_of_month(2023, 2); // Non-leap
        assert_eq!(d2.day(), 28);
    }

    #[test]
    fn last_day_of_december() {
        let d = last_day_of_month(2024, 12);
        assert_eq!(d.day(), 31);
    }

    #[test]
    fn model_pricing_cache_fields() {
        let p = get_model_pricing(Some("claude-opus-4-5-20251101"));
        assert_eq!(p.cache_write_per_mtok, Some(6.25));
        assert_eq!(p.cache_read_per_mtok, Some(0.5));

        let p2 = get_model_pricing(Some("claude-opus-4-20250514"));
        assert!(p2.cache_write_per_mtok.is_none());
    }
}
