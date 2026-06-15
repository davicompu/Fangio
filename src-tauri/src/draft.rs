//! Generates a Markdown SLR draft skeleton from project state.
//!
//! Everything here is deterministic — no model calls. The model is only used
//! for the optional "rewrite" or "expand" passes that the user can request
//! interactively in the editor (next session).

use crate::db::ProjectDb;
use anyhow::Result;

pub async fn generate(db: &ProjectDb) -> Result<String> {
    let project = db.get_project().await?;
    let prisma = db.prisma_state().await?;
    let included = db.list_included_articles_full().await?;
    let mut s = String::new();

    // Title
    let title = format!(
        "{}: a systematic review using {} ({})",
        project.name,
        project.framework.clone().unwrap_or_else(|| "framework".into()),
        project.slr_type.clone().unwrap_or_else(|| "domain".into()),
    );
    s.push_str(&format!("# {title}\n\n"));

    // Abstract
    s.push_str("## Abstract\n\n");
    s.push_str("**Background.** [TO COMPLETE — one paragraph motivating the review.]\n\n");
    if let Some(d) = project.domain.as_ref() {
        s.push_str("**Objective.** ");
        s.push_str(d);
        s.push_str("\n\n");
    }
    s.push_str("**Method.** This review follows the SPAR-4-SLR protocol (Paul et al., 2021) and reports its flow according to the PRISMA 2020 statement (Page et al., 2021). ");
    s.push_str(&format!(
        "Records were identified across multiple bibliographic databases ({} retrieved, {} retained after deduplication, {} included after title and abstract screening).",
        prisma.identified_databases,
        prisma.identified_databases - prisma.duplicates_removed,
        prisma.records_included
    ));
    s.push_str(" Included articles were coded against the ");
    s.push_str(&project.framework.clone().unwrap_or_else(|| "selected".into()));
    s.push_str(" framework.\n\n");
    s.push_str("**Results.** [TO COMPLETE — synthesise the dimension-by-dimension findings below.]\n\n");
    s.push_str("**Conclusion.** [TO COMPLETE — restate contributions and signal future agenda items.]\n\n");

    // Introduction
    s.push_str("## 1. Introduction\n\n");
    if let Some(d) = project.domain.as_ref() {
        s.push_str(&format!("This review investigates {d}\n\n"));
    }
    s.push_str("The review is guided by three research questions:\n\n");
    if let Some(q) = project.rq_what.as_ref() {
        s.push_str(&format!("**RQ1 (What).** {q}\n\n"));
    }
    if let Some(q) = project.rq_how.as_ref() {
        s.push_str(&format!("**RQ2 (How).** {q}\n\n"));
    }
    if let Some(q) = project.rq_where.as_ref() {
        s.push_str(&format!("**RQ3 (Where).** {q}\n\n"));
    }

    // Method
    s.push_str("## 2. Method\n\n");
    s.push_str("### 2.1 Protocol\n\n");
    s.push_str("This review follows the SPAR-4-SLR protocol (Paul et al., 2021) for the assembling, arranging, and assessing stages, and reports its identification, screening, and eligibility flow according to PRISMA 2020 (Page et al., 2021).\n\n");

    s.push_str("### 2.2 Search strategy\n\n");
    if let (Some(start), Some(end)) = (
        project.search_period_start.as_ref(),
        project.search_period_end.as_ref(),
    ) {
        s.push_str(&format!(
            "Searches were conducted across multiple bibliographic databases for publications dated {start}–{end}. "
        ));
    }
    s.push_str("[TO COMPLETE — list the exact databases used and the boolean query string from your assembling notes.]\n\n");

    s.push_str("### 2.3 PRISMA flow\n\n");
    s.push_str(&format!(
        "Database searches identified {} records. {} duplicate records were removed before screening, leaving {} records for title and abstract screening. {} records were excluded against pre-registered criteria, leaving {} included in the final review pool.\n\n",
        prisma.identified_databases,
        prisma.duplicates_removed,
        prisma.identified_databases - prisma.duplicates_removed,
        prisma.records_excluded,
        prisma.records_included,
    ));
    if !prisma.by_criterion.is_empty() {
        s.push_str("**Exclusions by criterion:**\n\n");
        for (label, n) in &prisma.by_criterion {
            if *n > 0 {
                s.push_str(&format!("- {label}: {n}\n"));
            }
        }
        s.push_str("\n");
    }

    s.push_str("### 2.4 Coding framework\n\n");
    if let Some(f) = project.framework.as_ref() {
        s.push_str(&format!(
            "Included articles were coded against the {f} framework. Each article was independently reviewed and tagged across every dimension of the framework.\n\n"
        ));
    }

    // Analysis method (SPAR-4-SLR Evaluation). Defaults to framework-based using
    // the chosen framework; only overridden when the researcher set one explicitly.
    s.push_str("### 2.5 Analysis method\n\n");
    let method = match project.analysis_method.as_ref().filter(|m| !m.trim().is_empty()) {
        Some(m) => m.trim().to_string(),
        None => match project.framework.as_deref() {
            Some("TCM") => "framework-based (TCM: Theories, Contexts, Methods)".to_string(),
            Some("ADO") => "framework-based (ADO: Antecedents, Decisions, Outcomes)".to_string(),
            Some(f) if !f.is_empty() && f != "Custom" => format!("framework-based ({f})"),
            _ => "framework-based".to_string(),
        },
    };
    s.push_str(&format!(
        "The synthesized corpus was analysed using a {method} analysis approach, with each included article coded across every dimension of the framework.\n\n"
    ));

    // Results
    s.push_str("## 3. Results\n\n");
    s.push_str(&format!(
        "The final corpus comprises {} articles. Tables of included articles, codings, and excluded articles are exported alongside this draft as CSV files.\n\n",
        included.len()
    ));
    match project.synthesis.as_ref().filter(|x| !x.trim().is_empty()) {
        Some(syn) => {
            s.push_str(syn.trim());
            s.push_str("\n\n");
        }
        None => s.push_str("[TO COMPLETE — generate a synthesis in Stage 3 → Synthesis, or write a short narrative for each dimension here.]\n\n"),
    }

    // Discussion
    s.push_str("## 4. Discussion\n\n");
    s.push_str("[TO COMPLETE — synthesise across dimensions and discuss theoretical, methodological, and practical implications.]\n\n");

    // Future agenda
    s.push_str("## 5. Future research agenda\n\n");
    match project.research_agenda.as_ref().filter(|a| !a.trim().is_empty()) {
        Some(agenda) => {
            s.push_str(agenda.trim());
            s.push_str("\n\n");
        }
        None => s.push_str("[TO COMPLETE — propose specific gaps to address. The Ask the corpus panel can help you identify contradictions and underexplored areas.]\n\n"),
    }

    // Limitations
    s.push_str("## 6. Limitations\n\n");
    if let Some(lim) = project.review_limitations.as_ref().filter(|l| !l.trim().is_empty()) {
        s.push_str(lim.trim());
        s.push_str("\n\n");
    } else {
        s.push_str("- Language bias: only English-language sources were included.\n");
        s.push_str("- Database coverage: any work indexed only in databases not searched is absent.\n");
        s.push_str("- Snapshot limitation: the search reflects the literature as of the search end date.\n");
        s.push_str("- Single-coder bias: coding was performed by one researcher with optional second-model assistance; full inter-rater reliability was not computed.\n\n");
    }

    // Conclusion
    s.push_str("## 7. Conclusion\n\n");
    s.push_str("[TO COMPLETE]\n\n");

    // Sources of support / funding (SPAR-4-SLR reporting)
    if let Some(funding) = project.sources_of_support.as_ref().filter(|f| !f.trim().is_empty()) {
        s.push_str("## Acknowledgements and funding\n\n");
        s.push_str(funding.trim());
        s.push_str("\n\n");
    }

    // References stub
    s.push_str("## References\n\n");
    s.push_str("*Generated by Reeves from the included corpus. The full machine-readable bibliography is exported as `data/included.bib`.*\n\n");
    for (i, a) in included.iter().enumerate() {
        let authors = a.authors.clone().unwrap_or_else(|| "Anonymous".into());
        let year = a.year.map(|y| y.to_string()).unwrap_or_else(|| "n.d.".into());
        let journal = a.journal.clone().unwrap_or_default();
        s.push_str(&format!(
            "{}. {} ({}). {}. *{}*.",
            i + 1,
            authors,
            year,
            a.title,
            journal,
        ));
        if let Some(doi) = a.doi.as_ref() {
            s.push_str(&format!(" https://doi.org/{doi}"));
        }
        s.push_str("\n");
    }

    Ok(s)
}

pub fn write_to_workspace(workspace: &std::path::Path, content: &str) -> Result<std::path::PathBuf> {
    let dir = workspace.join("drafts");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("slr_draft.md");
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Save a draft, but first archive any existing draft so the user can recover.
pub fn write_with_archive(
    workspace: &std::path::Path,
    content: &str,
) -> Result<std::path::PathBuf> {
    let dir = workspace.join("drafts");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("slr_draft.md");
    if path.exists() {
        let archive_dir = dir.join("archive");
        std::fs::create_dir_all(&archive_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
        let archive_path = archive_dir.join(format!("slr_draft.{stamp}.md"));
        std::fs::copy(&path, &archive_path)?;
    }
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Generate a PRISMA 2020 27-item checklist seeded with the project's
/// computable values; researcher prose marked [TO COMPLETE].
pub async fn generate_checklist(db: &ProjectDb) -> Result<String> {
    let project = db.get_project().await?;
    let prisma = db.prisma_state().await?;
    let mut s = String::new();
    s.push_str("# PRISMA 2020 checklist\n\n");
    s.push_str("Generated by Reeves. Items marked `[AUTO]` are filled from project state. Items marked `[TO COMPLETE]` need a paragraph or sentence from you. Reference: Page MJ, et al. The PRISMA 2020 statement: an updated guideline for reporting systematic reviews. BMJ 2021;372:n71.\n\n");
    s.push_str("| # | Section | Item | Reported on page / status |\n");
    s.push_str("|---|---|---|---|\n");

    let row = |n: &str, sec: &str, item: &str, status: &str, s: &mut String| {
        s.push_str(&format!("| {n} | {sec} | {item} | {status} |\n"));
    };
    let auto = "[AUTO]";
    let todo = "[TO COMPLETE]";

    row("1", "Title", "Identify the report as a systematic review", auto, &mut s);
    row("2", "Abstract", "See PRISMA-A 2020 abstract checklist", todo, &mut s);
    row("3", "Introduction · Rationale", "Describe the rationale for the review in the context of existing knowledge", todo, &mut s);
    row(
        "4",
        "Introduction · Objectives",
        "Provide an explicit statement of the objective(s) or question(s) the review addresses",
        if project.rq_what.is_some() { auto } else { todo },
        &mut s,
    );
    row("5", "Methods · Eligibility criteria", "Specify the inclusion and exclusion criteria for the review and how studies were grouped for the syntheses", auto, &mut s);
    row("6", "Methods · Information sources", "Specify all databases, registers, websites, organisations, reference lists and other sources searched or consulted", todo, &mut s);
    row("7", "Methods · Search strategy", "Present the full search strategies for all databases, registers and websites, including any filters and limits used", todo, &mut s);
    row("8", "Methods · Selection process", "Specify the methods used to decide whether a study met the inclusion criteria of the review", auto, &mut s);
    row("9", "Methods · Data collection process", "Specify the methods used to collect data from reports", todo, &mut s);
    row("10a", "Methods · Data items", "List and define all outcomes for which data were sought", todo, &mut s);
    row("10b", "Methods · Data items", "List and define all other variables for which data were sought", todo, &mut s);
    row("11", "Methods · Study risk of bias assessment", "Specify the methods used to assess risk of bias in the included studies", todo, &mut s);
    row("12", "Methods · Effect measures", "Specify for each outcome the effect measure(s) used in the synthesis or presentation of results", todo, &mut s);
    row("13a", "Methods · Synthesis methods", "Describe the processes used to decide which studies were eligible for each synthesis", auto, &mut s);
    row("13b", "Methods · Synthesis methods", "Describe any methods required to prepare the data for presentation or synthesis", todo, &mut s);
    row("13c", "Methods · Synthesis methods", "Describe any methods used to tabulate or visually display results of individual studies and syntheses", auto, &mut s);
    row("13d", "Methods · Synthesis methods", "Describe any methods used to synthesise results", todo, &mut s);
    row("14", "Methods · Reporting bias assessment", "Describe any methods used to assess risk of bias due to missing results in a synthesis", todo, &mut s);
    row("15", "Methods · Certainty assessment", "Describe any methods used to assess certainty (or confidence) in the body of evidence for an outcome", todo, &mut s);
    row(
        "16a",
        "Results · Study selection",
        &format!(
            "Describe the results of the search and selection process. Identified n = {}; deduplicated n = {}; included n = {}.",
            prisma.identified_databases,
            prisma.identified_databases - prisma.duplicates_removed,
            prisma.records_included
        ),
        auto,
        &mut s,
    );
    row("16b", "Results · Study selection", "Cite studies that might appear to meet the inclusion criteria, but which were excluded, and explain why they were excluded", todo, &mut s);
    row("17", "Results · Study characteristics", "Cite each included study and present its characteristics", auto, &mut s);
    row("18", "Results · Risk of bias in studies", "Present assessments of risk of bias for each included study", todo, &mut s);
    row("19", "Results · Results of individual studies", "For all outcomes, present, for each study, summary statistics for each group and an effect estimate and its precision", todo, &mut s);
    row("20a", "Results · Results of syntheses", "For each synthesis, briefly summarise the characteristics and risk of bias among contributing studies", todo, &mut s);
    row("20b", "Results · Results of syntheses", "Present results of all statistical syntheses conducted", todo, &mut s);
    row("20c", "Results · Results of syntheses", "Present results of all investigations of possible causes of heterogeneity among study results", todo, &mut s);
    row("20d", "Results · Results of syntheses", "Present results of all sensitivity analyses conducted to assess the robustness of the synthesised results", todo, &mut s);
    row("21", "Results · Reporting biases", "Present assessments of risk of bias due to missing results for each synthesis assessed", todo, &mut s);
    row("22", "Results · Certainty of evidence", "Present assessments of certainty in the body of evidence for each outcome assessed", todo, &mut s);
    row("23a", "Discussion", "Provide a general interpretation of the results in the context of other evidence", todo, &mut s);
    row("23b", "Discussion", "Discuss any limitations of the evidence included in the review", todo, &mut s);
    row("23c", "Discussion", "Discuss any limitations of the review processes used", todo, &mut s);
    row("23d", "Discussion", "Discuss implications of the results for practice, policy, and future research", todo, &mut s);
    row("24a", "Other · Registration and protocol", "Provide registration information for the review, including the register name and registration number, or state that the review was not registered", todo, &mut s);
    row("24b", "Other · Registration and protocol", "Indicate where the review protocol can be accessed, or state that a protocol was not prepared", todo, &mut s);
    row("24c", "Other · Registration and protocol", "Describe and explain any amendments to information provided at registration or in the protocol", todo, &mut s);
    row("25", "Other · Support", "Describe sources of financial or non-financial support for the review, and the role of the funders or sponsors in the review", todo, &mut s);
    row("26", "Other · Competing interests", "Declare any competing interests of review authors", todo, &mut s);
    row("27", "Other · Availability of data, code and other materials", "Report which of the following are publicly available and where they can be found: template data collection forms; data extracted from included studies; data used for all analyses; analytic code; any other materials used in the review", auto, &mut s);

    Ok(s)
}

pub fn write_checklist_to_workspace(
    workspace: &std::path::Path,
    content: &str,
) -> Result<std::path::PathBuf> {
    let dir = workspace.join("drafts");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("prisma_checklist.md");
    std::fs::write(&path, content)?;
    Ok(path)
}
