//! Clone-as-template. Copies a project's onboarding decisions, framework code
//! definitions, exclusion criteria, and settings into a fresh workspace —
//! leaving the article corpus, codings, screenings, PDFs, and decision log
//! behind. Useful for follow-up reviews in the same domain.

use crate::db::ProjectDb;
use anyhow::Result;
use std::path::Path;

pub async fn clone_template(
    source_workspace: &Path,
    dest_workspace: &Path,
    new_name: Option<String>,
) -> Result<ProjectDb> {
    let src_db = ProjectDb::open_or_create(source_workspace, None).await?;
    let project = src_db.get_project().await?;

    // Create the new workspace + bare project
    let dest = ProjectDb::open_or_create(
        dest_workspace,
        Some(new_name.clone().unwrap_or_else(|| format!("{} (clone)", project.name))),
    )
    .await?;

    // Copy project-level onboarding via decisions so the new project's
    // decision_log is properly seeded.
    let copy = |key: &str, val: Option<&String>| -> Option<crate::db::DecisionInput> {
        val.map(|v| crate::db::DecisionInput {
            stage: "onboarding".into(),
            sub_stage: None,
            decision_key: key.into(),
            value_json: serde_json::to_string(v).unwrap(),
            rationale: None,
        })
    };

    let decisions = vec![
        copy("project.slr_type", project.slr_type.as_ref()),
        copy("project.domain_subtype", project.domain_subtype.as_ref()),
        copy("project.framework", project.framework.as_ref()),
        copy("project.domain", project.domain.as_ref()),
        copy("project.rq_what", project.rq_what.as_ref()),
        copy("project.rq_how", project.rq_how.as_ref()),
        copy("project.rq_where", project.rq_where.as_ref()),
        copy("project.search_period_start", project.search_period_start.as_ref()),
        copy("project.search_period_end", project.search_period_end.as_ref()),
        copy("project.language_filter", project.language_filter.as_ref()),
        copy("project.quality_list", project.quality_list.as_ref()),
        copy("project.quality_threshold", project.quality_threshold.as_ref()),
    ];
    for d in decisions.into_iter().flatten() {
        dest.apply_decision(d).await?;
    }
    dest.apply_decision(crate::db::DecisionInput {
        stage: "onboarding".into(),
        sub_stage: None,
        decision_key: "project.onboarding_complete".into(),
        value_json: "true".into(),
        rationale: None,
    })
    .await?;

    // Copy framework code definitions
    let defs = src_db.list_framework_code_defs().await?;
    for d in defs {
        dest.add_framework_code_def(&d.dimension, &d.code, d.description.as_deref())
            .await?;
    }

    // Copy custom exclusion criteria (numbers > 9 = user-added; 1-9 are seeded by migration)
    let crits = src_db.list_criteria().await?;
    for c in crits {
        if c.number > 9 {
            dest.save_criterion(None, &c.label, c.rationale.as_deref(), c.is_deferred_quality_filter)
                .await?;
        }
    }

    // Copy LM Studio + telemetry settings
    let s = src_db.all_settings().await?;
    if let serde_json::Value::Object(map) = s {
        for (k, v) in map {
            if k.starts_with("lmstudio.") || k.starts_with("telemetry.") {
                dest.set_setting(&k, &v).await?;
            }
        }
    }

    Ok(dest)
}
