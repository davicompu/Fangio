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
        prisma.records_screened,
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
        prisma.records_screened,
        prisma.records_excluded,
        prisma.records_included,
    ));
    // Flow as a compact table so it reads cleanly in the draft and the DOCX.
    s.push_str("| PRISMA stage | Records |\n|---|---|\n");
    s.push_str(&format!("| Identified (databases) | {} |\n", prisma.identified_databases));
    s.push_str(&format!("| Duplicates removed | {} |\n", prisma.duplicates_removed));
    s.push_str(&format!("| Screened | {} |\n", prisma.records_screened));
    s.push_str(&format!("| Excluded | {} |\n", prisma.records_excluded));
    s.push_str(&format!("| Included | {} |\n\n", prisma.records_included));
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

/// Read the saved draft markdown if one exists (empty string when none).
pub fn read_from_workspace(workspace: &std::path::Path) -> Result<String> {
    let path = workspace.join("drafts").join("slr_draft.md");
    if path.exists() {
        Ok(std::fs::read_to_string(&path)?)
    } else {
        Ok(String::new())
    }
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
            prisma.records_screened,
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

// ── DOCX export ─────────────────────────────────────────────────────────────
// Convert the manuscript markdown subset (headings, paragraphs, bullets, bold,
// italic, and pipe tables) into a Word document so PRISMA and findings tables
// render as real tables rather than raw markdown.

use docx_rs::{Docx, Paragraph, Run, Table, TableCell, TableRow, WidthType};

/// Is there a closing marker for `c` ahead of `start`? (`double` = look for `cc`).
fn has_closing_marker(chars: &[char], start: usize, c: char, double: bool) -> bool {
    let mut j = start;
    while j < chars.len() {
        if double {
            if j + 1 < chars.len() && chars[j] == c && chars[j + 1] == c {
                return true;
            }
        } else if chars[j] == c {
            return true;
        }
        j += 1;
    }
    false
}

/// Split inline markdown into (text, bold, italic) runs. Handles **bold**,
/// *italic*/_italic_, and drops backticks. A marker only toggles emphasis when a
/// matching closing marker exists ahead — so a lone `*` or `_` (e.g. "max_value",
/// "5*3") is treated as a literal character instead of flipping the rest of the line.
fn parse_runs(text: &str) -> Vec<(String, bool, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let mut runs: Vec<(String, bool, bool)> = Vec::new();
    let mut buf = String::new();
    let (mut bold, mut italic) = (false, false);
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            // Close an open bold, or open one only if it closes later.
            if bold || has_closing_marker(&chars, i + 2, '*', true) {
                if !buf.is_empty() { runs.push((std::mem::take(&mut buf), bold, italic)); }
                bold = !bold;
                i += 2;
                continue;
            }
            buf.push('*');
            buf.push('*');
            i += 2;
            continue;
        }
        let c = chars[i];
        if c == '*' || c == '_' {
            if italic || has_closing_marker(&chars, i + 1, c, false) {
                if !buf.is_empty() { runs.push((std::mem::take(&mut buf), bold, italic)); }
                italic = !italic;
                i += 1;
                continue;
            }
            buf.push(c);
            i += 1;
            continue;
        }
        if c == '`' { i += 1; continue; }
        buf.push(c);
        i += 1;
    }
    if !buf.is_empty() { runs.push((buf, bold, italic)); }
    runs
}

#[cfg(test)]
mod parse_runs_tests {
    use super::parse_runs;

    #[test]
    fn lone_markers_are_literal() {
        assert_eq!(parse_runs("max_value here"), vec![("max_value here".to_string(), false, false)]);
        assert_eq!(parse_runs("5 * 3 = 15"), vec![("5 * 3 = 15".to_string(), false, false)]);
    }

    #[test]
    fn paired_markers_toggle() {
        let r = parse_runs("a **b** c");
        assert_eq!(r, vec![
            ("a ".into(), false, false),
            ("b".into(), true, false),
            (" c".into(), false, false),
        ]);
    }

    #[test]
    fn paired_italic_toggles() {
        let r = parse_runs("x _y_ z");
        assert_eq!(r, vec![
            ("x ".into(), false, false),
            ("y".into(), false, true),
            (" z".into(), false, false),
        ]);
    }
}

fn para(text: &str, size: usize, force_bold: bool) -> Paragraph {
    let mut p = Paragraph::new();
    for (t, b, it) in parse_runs(text) {
        let mut r = Run::new().add_text(t).size(size);
        if b || force_bold { r = r.bold(); }
        if it { r = r.italic(); }
        p = p.add_run(r);
    }
    p
}

fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
}

fn md_table(block: &[&str]) -> Table {
    let mut rows: Vec<TableRow> = Vec::new();
    let mut first = true;
    for line in block {
        let cells = split_cells(line);
        if is_separator_row(&cells) { continue; }
        let bold = first;
        first = false;
        let tcells: Vec<TableCell> = cells
            .iter()
            .map(|c| TableCell::new().add_paragraph(para(c, 20, bold)))
            .collect();
        rows.push(TableRow::new(tcells));
    }
    Table::new(rows).width(9638, WidthType::Dxa)
}

/// Build a Word document from the manuscript markdown subset.
pub fn markdown_to_docx(md: &str) -> Docx {
    let mut docx = Docx::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if line.trim_start().starts_with('|') {
            let mut block: Vec<&str> = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                block.push(lines[i]);
                i += 1;
            }
            docx = docx.add_table(md_table(&block));
            docx = docx.add_paragraph(Paragraph::new());
            continue;
        }
        if line.trim().is_empty() {
            docx = docx.add_paragraph(Paragraph::new());
        } else if let Some(rest) = line.strip_prefix("### ") {
            docx = docx.add_paragraph(para(rest, 26, true));
        } else if let Some(rest) = line.strip_prefix("## ") {
            docx = docx.add_paragraph(para(rest, 30, true));
        } else if let Some(rest) = line.strip_prefix("# ") {
            docx = docx.add_paragraph(para(rest, 36, true));
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            docx = docx.add_paragraph(para(&format!("•  {rest}"), 22, false));
        } else {
            docx = docx.add_paragraph(para(line, 22, false));
        }
        i += 1;
    }
    docx
}

// ── Decision report ─────────────────────────────────────────────────────────
// A human-readable audit trail of the review's decisions, compiled from the
// decision log (configuration), the article table (screening), the coding table
// (coding), and the AI-resolution tables — labelling human vs AI throughout.

fn cell(s: &str) -> String {
    // Markdown table cells can't contain raw '|' or newlines.
    s.replace('|', "/").replace('\n', " ").trim().to_string()
}

fn short(s: &str, n: usize) -> String {
    let t = s.trim();
    if t.chars().count() > n {
        format!("{}…", t.chars().take(n).collect::<String>())
    } else {
        t.to_string()
    }
}

fn decode_value(value_json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(value_json) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(serde_json::Value::Array(a)) => a
            .iter()
            .map(|v| v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string()))
            .collect::<Vec<_>>()
            .join(", "),
        Ok(other) => other.to_string(),
        Err(_) => value_json.to_string(),
    }
}

fn key_label(key: &str) -> String {
    key.strip_prefix("project.").unwrap_or(key).replace('_', " ")
}

fn setup_step_label(step: &str) -> &str {
    match step {
        "name" => "Title",
        "slr_type" => "Review type",
        "framework" => "Framework",
        "domain" => "Domain",
        "rq_what" => "Research question: What do we know?",
        "rq_how" => "Research question: How do we know?",
        "rq_where" => "Research question: Where should the field head?",
        other => other,
    }
}

fn setup_outcome_label(outcome: &str) -> &str {
    match outcome {
        "used" => "Used the suggestion",
        "undone" => "Undid the suggestion",
        "ignored" => "Ignored the suggestion and kept their own answer",
        "kept_choice" => "Kept their own choice",
        "changed_choice" => "Changed their choice after the advice",
        "closed_before_suggestion" => "Closed the assistant before a suggestion was made",
        other => other,
    }
}

/// One line of text for a report bullet: no newlines, quoted.
fn quoted(s: &str) -> String {
    format!("\u{201c}{}\u{201d}", s.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// §1a: every use of the setup assistant, grouped by call, in the order asked.
fn setup_assistant_section(log: &[crate::db::DecisionLogRow]) -> String {
    use crate::setup_assistant::{KEY_OUTCOME, KEY_PREFIX, KEY_QUESTIONS, KEY_SUGGESTION};
    let mut s = String::from("## 1a. Setup assistant\n\n");

    // call_id → (first timestamp, entries in order)
    let mut order: Vec<String> = Vec::new();
    let mut calls: std::collections::HashMap<String, Vec<(&crate::db::DecisionLogRow, serde_json::Value)>> =
        std::collections::HashMap::new();
    for d in log.iter().filter(|d| d.decision_key.starts_with(KEY_PREFIX)) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&d.value_json) else { continue };
        let Some(id) = v["call_id"].as_str().map(str::to_string) else { continue };
        if !calls.contains_key(&id) {
            order.push(id.clone());
        }
        calls.entry(id).or_default().push((d, v));
    }
    if order.is_empty() {
        s.push_str("_The setup assistant was not used._\n\n");
        return s;
    }
    s.push_str("Each use of \u{201c}Ask assistant\u{201d} on the setup answers, during setup or when editing them later from the stage overview. Lines marked **AI** are the assistant's output; lines marked **Researcher** are the researcher's input or actions. The researcher's final answers are the decisions in section 1.\n\n");

    for id in &order {
        let entries = &calls[id];
        let (first, first_v) = &entries[0];
        let step = first_v["step"].as_str().unwrap_or("");
        s.push_str(&format!(
            "**{} \u{2014} {}**\n\n",
            setup_step_label(step),
            crate::transparency::utc_time(&first.created_at)
        ));
        let questions = entries.iter().find(|(d, _)| d.decision_key == KEY_QUESTIONS);
        let suggestion = entries.iter().find(|(d, _)| d.decision_key == KEY_SUGGESTION);

        let field_text = first_v["field_text"].as_str().unwrap_or("");
        s.push_str(&format!(
            "- Researcher \u{2014} text in the field when asking: {}\n",
            if field_text.trim().is_empty() { "(empty)".to_string() } else { quoted(field_text) }
        ));
        let context: Vec<String> = first_v["context"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| format!("{}: {}", c["label"].as_str().unwrap_or(""), quoted(c["value"].as_str().unwrap_or(""))))
                    .collect()
            })
            .unwrap_or_default();
        s.push_str(&format!(
            "- Earlier setup answers sent as context: {}\n",
            if context.is_empty() { "none".to_string() } else { context.join("; ") }
        ));

        if let Some((_, q)) = questions {
            let model = q["model"].as_str().unwrap_or("\u{2014}");
            if let Some(err) = q["error"].as_str() {
                s.push_str(&format!("- AI \u{2014} clarifying questions failed ({model}): {}\n", quoted(err)));
            } else {
                let qs: Vec<String> = q["questions"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|x| {
                                let opts: Vec<&str> = x["options"]
                                    .as_array()
                                    .map(|o| o.iter().filter_map(|v| v.as_str()).collect())
                                    .unwrap_or_default();
                                if opts.is_empty() {
                                    quoted(x["q"].as_str().unwrap_or(""))
                                } else {
                                    format!("{} (options: {})", quoted(x["q"].as_str().unwrap_or("")), opts.join(" / "))
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                s.push_str(&format!("- AI \u{2014} clarifying questions ({model}): {}\n", qs.join("; ")));
            }
        }

        if let Some((d, v)) = suggestion {
            if v["skipped"].as_bool().unwrap_or(false) {
                s.push_str("- Researcher \u{2014} skipped the questions (\u{201c}Skip \u{2014} just suggest\u{201d})\n");
            } else {
                let answers: Vec<String> = v["answers"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|x| format!("{} \u{2192} {}", quoted(x["q"].as_str().unwrap_or("")), quoted(x["a"].as_str().unwrap_or(""))))
                            .collect()
                    })
                    .unwrap_or_default();
                if !answers.is_empty() {
                    s.push_str(&format!("- Researcher \u{2014} answers: {}\n", answers.join("; ")));
                }
            }
            let model = v["model"].as_str().unwrap_or("\u{2014}");
            if let Some(err) = v["error"].as_str() {
                s.push_str(&format!("- AI \u{2014} suggestion failed ({model}): {}\n", quoted(err)));
            } else {
                s.push_str(&format!(
                    "- AI \u{2014} suggestion ({model}): {}\n",
                    quoted(v["suggestion"].as_str().unwrap_or(""))
                ));
                if let Some(r) = d.rationale.as_deref().filter(|r| !r.trim().is_empty()) {
                    s.push_str(&format!("- AI \u{2014} reason: {}\n", quoted(r)));
                }
            }
        }

        let outcomes: Vec<&serde_json::Value> = entries
            .iter()
            .filter(|(d, _)| d.decision_key == KEY_OUTCOME)
            .map(|(_, v)| v)
            .collect();
        if outcomes.is_empty() {
            s.push_str("- Researcher \u{2014} next action: not recorded\n");
        } else {
            let steps: Vec<&str> = outcomes.iter().map(|v| setup_outcome_label(v["outcome"].as_str().unwrap_or(""))).collect();
            let final_value = outcomes.iter().rev().find_map(|v| v["final_value"].as_str());
            s.push_str(&format!(
                "- Researcher \u{2014} next action: {}{}\n",
                steps.join(" \u{2192} "),
                final_value.map(|f| format!(" (answer at that point: {})", quoted(f))).unwrap_or_default()
            ));
        }
        s.push('\n');
    }
    s
}

pub async fn generate_decision_report(db: &ProjectDb) -> Result<String> {
    let project = db.get_project().await?;
    let log = db.list_decision_log().await?;
    let articles = db.list_articles().await?;
    let criteria = db.list_criteria().await?;
    let ids: Vec<i64> = articles.iter().map(|a| a.id).collect();
    let codings = db.list_codings_for_articles(&ids).await?;
    let adjudications = db.list_adjudications_for_articles(&ids).await?;
    let consolidations = db.list_all_consolidations().await?;
    let review_events = db.list_review_events().await?;
    let fulltext_status = db.fulltext_status_map().await?;
    let ai_log = db.list_ai_screening_log(None, None).await?;
    let ai_calls = db.list_ai_calls().await?;
    let call_cells = |id: Option<i64>| -> (String, String, String) {
        match id.and_then(|i| ai_calls.get(&i)) {
            Some(c) => (
                c.model_used.clone().unwrap_or_else(|| c.model_requested.clone()),
                c.prompt_hash.clone().unwrap_or_else(|| "—".into()),
                crate::transparency::format_temperature(c.temperature),
            ),
            None => ("—".into(), "—".into(), "—".into()),
        }
    };
    let pct = |c: Option<f64>| c.map(|v| format!("{}%", (v * 100.0).round() as i64)).unwrap_or_else(|| "—".into());
    let stage_name = |st: &str| if st == "fulltext" { "Full text" } else { "Title & abstract" };
    // Latest AI decision's review status per (article, stage).
    let latest_status: std::collections::HashMap<(i64, &str), &str> = ai_log
        .iter()
        .filter(|e| e.is_latest)
        .map(|e| ((e.article_id, e.stage.as_str()), crate::transparency::status_label(e.review_status.as_deref())))
        .collect();

    // Latest real (include/exclude/uncertain) AI row per (article, stage).
    let mut latest_real: std::collections::HashMap<(i64, &str), &crate::db::AiScreeningLogEntry> =
        std::collections::HashMap::new();
    for e in &ai_log {
        if matches!(e.decision.as_str(), "include" | "exclude" | "uncertain") {
            let k = (e.article_id, e.stage.as_str());
            if latest_real.get(&k).map_or(true, |p| p.id < e.id) {
                latest_real.insert(k, e);
            }
        }
    }
    let logged_articles: std::collections::HashSet<i64> = ai_log.iter().map(|e| e.article_id).collect();

    let crit_label: std::collections::HashMap<i64, String> =
        criteria.iter().map(|c| (c.number, c.label.clone())).collect();
    let title_of: std::collections::HashMap<i64, String> =
        articles.iter().map(|a| (a.id, a.title.clone())).collect();

    let mut s = String::new();
    s.push_str(&format!("# Decision report — {}\n\n", project.name));
    s.push_str("This report documents the decisions made during the review for transparency and replication. Each decision is labelled by who made it — the **researcher (human)** or an **AI assistant** whose suggestion was applied. It is generated from the project's decision log and database.\n\n");

    // 1. Configuration decisions (human, via the decision log)
    s.push_str("## 1. Configuration decisions\n\n");
    // Setup-assistant entries are reported in §1a, not as protocol decisions.
    let protocol: Vec<&crate::db::DecisionLogRow> = log
        .iter()
        .filter(|d| !d.decision_key.starts_with(crate::setup_assistant::KEY_PREFIX))
        .collect();
    let effective: Vec<&&crate::db::DecisionLogRow> = protocol.iter().filter(|d| d.superseded_by.is_none()).collect();
    if effective.is_empty() {
        s.push_str("_No configuration decisions recorded._\n\n");
    } else {
        s.push_str("These are protocol decisions (research questions, framework, search strategy, eligibility criteria, and Stage-3 outputs), all made by the researcher.\n\n");
        s.push_str("| When | Stage | Decision | Value |\n|---|---|---|---|\n");
        for d in &effective {
            s.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                cell(&crate::transparency::utc_time(&d.created_at)),
                cell(&d.stage),
                cell(&key_label(&d.decision_key)),
                cell(&short(&decode_value(&d.value_json), 100)),
            ));
        }
        let revised = protocol.len() - effective.len();
        if revised > 0 {
            s.push_str(&format!("\n_{revised} earlier decision(s) were later revised and are superseded above._\n"));
        }
        s.push_str("\n");
    }

    s.push_str(&setup_assistant_section(&log));

    // 2. Screening decisions (human or AI)
    s.push_str("## 2. Screening decisions\n\n");
    let screened: Vec<_> = articles.iter().filter(|a| a.status == "included" || a.status == "excluded").collect();
    if screened.is_empty() {
        s.push_str("_No screening decisions recorded yet._\n\n");
    } else {
        s.push_str("| Article | Decision | Stage | Criterion | Decided by | Review status | When |\n|---|---|---|---|---|---|---|\n");
        for a in &screened {
            // Which screening pass produced the final decision, derived from the
            // article's full-text status: a full-text exclusion/inclusion is
            // marked on `fulltext_status`; otherwise the call was at title/abstract.
            let ft = fulltext_status.get(&a.id).cloned().flatten();
            let stage = match (a.status.as_str(), ft.as_deref()) {
                ("excluded", Some("excluded")) => "Full text",
                ("included", Some("included")) => "Full text",
                _ => "Title & abstract",
            };
            let st_key = if stage == "Full text" { "fulltext" } else { "title_abstract" };
            let human_or_auto = if a.screened_at.is_some() { "Human" } else { "Auto (PDF attached)" };
            // Decided by the AI only when the logged decision for that stage was
            // applied automatically, still stands, and matches the article.
            let decided_by = match latest_real.get(&(a.id, st_key)) {
                Some(e) => {
                    let matches_article = (e.decision == "include" && a.status == "included")
                        || (e.decision == "exclude" && a.status == "excluded");
                    if e.auto_applied && e.overridden_at.is_none() && matches_article {
                        "AI"
                    } else {
                        human_or_auto
                    }
                }
                // No log row at that stage (decisions made before the log existed).
                None if a.ai_rationale.is_some() && !logged_articles.contains(&a.id) => "AI",
                None => human_or_auto,
            };
            let criterion = if a.status == "excluded" {
                a.exclusion_criterion
                    .and_then(|n| crit_label.get(&n).cloned().or(Some(format!("Criterion {n}"))))
                    .unwrap_or_else(|| "—".into())
            } else {
                "—".into()
            };
            let review_status = latest_status.get(&(a.id, st_key)).copied().unwrap_or("—");
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                cell(&short(&a.title, 60)),
                if a.status == "included" { "Included" } else { "Excluded" },
                stage,
                cell(&criterion),
                decided_by,
                review_status,
                cell(&a.screened_at.as_deref().map(crate::transparency::utc_time).unwrap_or_else(|| "—".into())),
            ));
        }
        s.push_str("\n");
    }

    // 2a. Full AI screening log
    s.push_str("## 2a. AI screening log\n\n");
    // Decisions whose review status can't be determined: latest logged decisions
    // decided outside the log, plus AI-decided articles with no log row at all.
    let undetermined_logged = ai_log
        .iter()
        .filter(|e| e.is_latest && e.review_status.is_none() && !matches!(e.decision.as_str(), "error" | "skipped"))
        .count();
    let unlogged = articles
        .iter()
        .filter(|a| a.ai_rationale.is_some() && (a.status == "included" || a.status == "excluded"))
        .filter(|a| !logged_articles.contains(&a.id))
        .count();
    let undetermined_note = match undetermined_logged + unlogged {
        0 => None,
        1 => Some("1 earlier AI screening decision was recorded before this log existed, or was decided outside it; its review status can't be determined.\n\n".to_string()),
        n => Some(format!("{n} earlier AI screening decisions were recorded before this log existed, or were decided outside it; their review status can't be determined.\n\n")),
    };
    if ai_log.is_empty() {
        s.push_str("_No AI screening decisions recorded._\n\n");
        if let Some(note) = &undetermined_note {
            s.push_str(note);
        }
    } else {
        s.push_str("AI screening decisions recorded in this log, including earlier ones that were later replaced.\n\n");
        s.push_str("| Article | Stage | Mode | AI decision | Criterion | Confidence | Model used | Prompt fingerprint | Temperature | Applied automatically | Review status | Opened |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for e in &ai_log {
            let model = e.model_used.clone().or_else(|| e.model_name.clone()).unwrap_or_else(|| "—".into());
            let crit = match e.criterion_number {
                Some(n) => crit_label.get(&n).cloned().unwrap_or_else(|| format!("Criterion {n}")),
                None => "—".into(),
            };
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                cell(&short(&e.article_title, 60)),
                stage_name(&e.stage),
                if e.mode == "purification" { "Purification" } else { "Screening" },
                cell(&capitalize(&e.decision)),
                cell(&crit),
                pct(e.confidence),
                cell(&model),
                cell(e.prompt_hash.as_deref().unwrap_or("—")),
                e.temperature.map(crate::transparency::format_temperature).unwrap_or_else(|| "—".into()),
                if e.auto_applied { "Yes" } else { "No" },
                crate::transparency::status_label(e.review_status.as_deref()),
                cell(&crate::transparency::opens_cell(
                    e.open_count,
                    e.first_opened_at.as_deref(),
                    e.last_opened_at.as_deref(),
                    e.last_opened_by.as_deref(),
                )),
            ));
        }
        if let Some(note) = &undetermined_note {
            s.push_str("\n");
            s.push_str(note.trim_end());
            s.push_str("\n");
        }
        let mut seeds: Vec<i64> = ai_calls
            .values()
            .filter(|c| c.purpose == "screening_ta" || c.purpose == "screening_ft")
            .filter_map(|c| c.seed)
            .collect();
        seeds.sort();
        seeds.dedup();
        s.push_str("\n");
        match seeds.len() {
            0 => s.push_str("Seed: not set.\n\n"),
            1 => s.push_str(&format!("Seed: {}\n\n", seeds[0])),
            _ => s.push_str(&format!(
                "Seeds: {}\n\n",
                seeds.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
            )),
        }
        s.push_str("Recorded to help others re-run the analysis; model outputs may still vary between runs.\n\n");
    }

    // 2b. Auto-applied decisions
    s.push_str("## 2b. Applied automatically\n\n");
    let auto: Vec<_> = ai_log.iter().filter(|e| e.is_latest && e.auto_applied).collect();
    if auto.is_empty() {
        s.push_str("_No decisions were applied automatically._\n\n");
    } else {
        for e in &auto {
            let verb = match e.decision.as_str() {
                "include" => "Included".to_string(),
                "exclude" => "Excluded".to_string(),
                other => capitalize(other),
            };
            let mut detail = format!(
                "{}, {}",
                pct(e.confidence),
                crate::transparency::opens_cell(
                    e.open_count,
                    e.first_opened_at.as_deref(),
                    e.last_opened_at.as_deref(),
                    e.last_opened_by.as_deref(),
                )
            );
            if e.review_status.as_deref() == Some("overridden") {
                detail.push_str(", Overridden");
            }
            s.push_str(&format!("- {} — {} ({})\n", verb, cell(&short(&e.article_title, 80)), detail));
        }
        s.push_str("\n");
    }

    // 3. Coding decisions (human vs AI)
    s.push_str("## 3. Coding decisions\n\n");
    if codings.is_empty() {
        s.push_str("_No codes applied yet._\n\n");
    } else {
        s.push_str("| Article | Dimension | Code | Applied by | Confidence (rater 1 / rater 2) | Reasons | Model used | Prompt fingerprint | Temperature | When |\n|---|---|---|---|---|---|---|---|---|---|\n");
        for c in &codings {
            let by = match c.source.as_str() {
                "researcher" => "Human",
                s if s.starts_with("model-suggested") => "Human (AI-suggested)",
                s if s.starts_with("model-ai") => "AI",
                other => other,
            };
            let title = title_of.get(&c.article_id).cloned().unwrap_or_else(|| format!("#{}", c.article_id));
            let conf = if c.confidence2.is_some() {
                format!("{} / {}", pct(c.confidence), pct(c.confidence2))
            } else {
                pct(c.confidence)
            };
            let reasons = match (c.rationale.as_deref(), c.rationale2.as_deref()) {
                (Some(r1), Some(r2)) => format!("R1: {} / R2: {}", short(r1, 120), short(r2, 120)),
                (Some(r1), None) => format!("R1: {}", short(r1, 120)),
                (None, Some(r2)) => format!("R2: {}", short(r2, 120)),
                (None, None) => "—".into(),
            };
            let m1 = c.model_used.clone().or_else(|| c.model_name.clone());
            let m2 = c.model_used2.clone().or_else(|| c.model_name2.clone());
            let model = match (m1, m2) {
                (Some(a), Some(b)) if a != b => format!("{a} / {b}"),
                (Some(a), _) => a,
                (None, Some(b)) => b,
                (None, None) => "—".into(),
            };
            let both = |a: Option<String>, b: Option<String>| match (a, b) {
                (Some(a), Some(b)) if a != b => format!("{a} / {b}"),
                (Some(a), _) => a,
                (None, Some(b)) => b,
                (None, None) => "—".into(),
            };
            let hash = both(c.prompt_hash.clone(), c.prompt_hash2.clone());
            let temp = both(
                c.temperature.map(crate::transparency::format_temperature),
                c.temperature2.map(crate::transparency::format_temperature),
            );
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                cell(&short(&title, 48)),
                cell(&c.dimension),
                cell(&c.code),
                by,
                conf,
                cell(&reasons),
                cell(&model),
                cell(&hash),
                temp,
                cell(&crate::transparency::utc_time(&c.created_at)),
            ));
        }
        s.push_str("\nConfidence is the model's own estimate, not a measured probability.\n\n");
    }

    // 4. Merging agent and consolidation
    s.push_str("## 4. Merging agent and consolidation\n\n");
    if adjudications.is_empty() && consolidations.is_empty() {
        s.push_str("_No AI merges or consolidations._\n\n");
    } else {
        if !adjudications.is_empty() {
            s.push_str("**Merges** (a merging agent combined two raters' equivalent codes):\n\n");
            s.push_str("| Dimension | Kept | Merged from | Reason | Model used | Prompt fingerprint | Temperature |\n|---|---|---|---|---|---|---|\n");
            for a in &adjudications {
                let (m, h, t) = call_cells(a.ai_call_id);
                let kept = if a.proposal_id.is_some() && a.coding_id.is_none() {
                    format!("{} (suggestion)", a.canonical_code)
                } else {
                    a.canonical_code.clone()
                };
                s.push_str(&format!(
                    "| {} | {} | {} + {} | {} | {} | {} | {} |\n",
                    cell(&a.dimension),
                    cell(&kept),
                    cell(&a.label1),
                    cell(&a.label2),
                    cell(&short(a.adjudicator_rationale.as_deref().unwrap_or("—"), 120)),
                    cell(&m),
                    cell(&h),
                    t,
                ));
            }
            s.push_str("\n");
        }
        if !consolidations.is_empty() {
            s.push_str("**Consolidated codes** (synonymous codes merged corpus-wide):\n\n");
            s.push_str("| Dimension | Canonical | Merged variant | Count | Model used | Prompt fingerprint | Temperature |\n|---|---|---|---|---|---|---|\n");
            for c in &consolidations {
                let (m, h, t) = call_cells(c.ai_call_id);
                s.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} |\n",
                    cell(&c.dimension),
                    cell(&c.canonical_code),
                    cell(&c.variant_code),
                    c.count,
                    cell(&m),
                    cell(&h),
                    t,
                ));
            }
            s.push_str("\n");
        }
    }

    // 5. Review actions (rejects, dedup merges — the forward trail)
    s.push_str("## 5. Review actions\n\n");
    if review_events.is_empty() {
        s.push_str("_No review actions recorded yet (reverts, resets, accepted and rejected suggestions, undone merges, reminder choices and deduplication merges are logged here)._\n\n");
    } else {
        s.push_str("Recorded actions include reverts, resets, accepted and rejected suggestions, undone merges, reminder choices and deduplication merges.\n\n");
        s.push_str("| When | Stage | Action | Detail | By |\n|---|---|---|---|---|\n");
        for e in &review_events {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cell(&crate::transparency::utc_time(&e.created_at)),
                cell(&e.stage),
                cell(&e.action.replace('_', " ")),
                cell(&short(e.detail.as_deref().unwrap_or("—"), 80)),
                cell(&capitalize(&e.actor)),
            ));
        }
        s.push_str("\n");
    }

    Ok(s)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod report_tests {
    use super::*;
    use crate::db::{test_db, NewArticle, NewScreeningDecision, ProjectDb};
    use crate::suggest::CallMeta;

    #[tokio::test]
    async fn report_lists_ai_log_and_auto_applied() {
        let (db, _d) = test_db().await;
        let mut ids = vec![];
        for t in ["Alpha study", "Beta study"] {
            ids.push(
                db.insert_article(&NewArticle {
                    title: t.into(),
                    authors: None,
                    year: None,
                    journal: None,
                    abstract_text: None,
                    doi: None,
                    url: None,
                    source_databases: vec![],
                    import_batches: vec![],
                })
                .await
                .unwrap(),
            );
        }
        let call = db
            .insert_ai_call(
                "screening_ta",
                &CallMeta {
                    model_requested: "m".into(),
                    model_used: "m-q4".into(),
                    prompt_hash: "abcdef123456".into(),
                    temperature: 0.2,
                    seed: None,
                },
            )
            .await
            .unwrap();
        let mut props = vec![];
        for id in &ids {
            props.push(
                db.save_ai_screening_decision(&NewScreeningDecision {
                    article_id: *id,
                    stage: "title_abstract",
                    mode: "screening",
                    decision: "exclude",
                    criterion_number: None,
                    rationale: "off topic",
                    confidence: Some(0.93),
                    model_name: Some("m"),
                    ai_call_id: Some(call),
                    auto_applied: true,
                    run_id: "r1",
                })
                .await
                .unwrap(),
            );
        }
        db.record_ai_screening_open(props[0], "tester", "test").await.unwrap();
        db.record_ai_screening_open(props[0], "tester", "test").await.unwrap();

        let mk = |h: &str, t: f32, seed: Option<i64>| CallMeta {
            model_requested: "c".into(),
            model_used: "c-used".into(),
            prompt_hash: h.into(),
            temperature: t,
            seed,
        };
        let c1 = db.insert_ai_call("coding", &mk("aaaaaaaaaaaa", 0.2, Some(7))).await.unwrap();
        let c2 = db.insert_ai_call("coding", &mk("bbbbbbbbbbbb", 0.5, None)).await.unwrap();
        let mc = db.insert_ai_call("merging", &mk("cccccccccccc", 0.3, None)).await.unwrap();
        let vote = |m: &str, id: i64| crate::transparency::AiCodeVote {
            rationale: Some("why".into()),
            model_name: Some(m.into()),
            confidence: Some(0.8),
            ai_call_id: Some(id),
        };
        db.save_ai_coding(ids[0], "Dim", "Code", "model-ai-consensus", &vote("m1", c1), Some(&vote("m2", c2)))
            .await
            .unwrap();
        let prop = |rater: i64, code: &str| crate::db::AiCodingProposal {
            id: 0,
            article_id: ids[0],
            dimension: "Dim".into(),
            code: code.into(),
            rationale: None,
            rater,
            model_name: None,
            run_id: "r".into(),
            created_at: "".into(),
            agreement: "one".into(),
            ai_call_id: None,
            ai_call_id2: None,
            rationale2: None,
            model_name2: None,
            confidence: None,
            confidence2: None,
            model_used: None,
            model_used2: None,
            prompt_hash: None,
            temperature: None,
        };
        let (p1, p2) = (prop(1, "x1"), prop(2, "x2"));
        db.insert_adjudication(&crate::db::NewAdjudication {
            article_id: ids[0],
            dimension: "Dim",
            canonical_code: "MergedKept",
            coding_id: None,
            proposal_id: Some(99),
            p1: &p1,
            p2: &p2,
            adjudicator_rationale: Some("same idea"),
            ai_call_id: Some(mc),
            run_id: "r",
        })
        .await
        .unwrap();

        let md = generate_decision_report(&db).await.unwrap();
        assert!(md.contains("aaaaaaaaaaaa / bbbbbbbbbbbb"));
        assert!(md.contains("0.2 / 0.5"));
        assert!(md.contains("MergedKept (suggestion)"));
        assert!(md.contains("cccccccccccc"));
        assert!(md.contains("## 2a. AI screening log"));
        assert!(md.contains("Never opened"));
        assert!(md.contains("2 (first"));
        assert!(md.contains("abcdef123456"));
        assert!(md.contains("| 0.2 |"));
        assert!(md.contains("93%"));
        assert!(md.contains("Seed: not set."));
        assert!(md.contains("## 2b. Applied automatically"));
        assert!(md.contains("Excluded — Beta study (93%, Never opened)"));
        assert!(md.contains("## 4. Merging agent and consolidation"));
        assert!(!md.to_lowercase().contains("adjudicat"));
        assert!(!md.contains("reviewed"));
        assert!(!md.contains("reproducible"));
        assert!(md.contains("UTC"));
        assert!(md.contains("last opened by tester"));
        assert!(!md.contains("earlier AI screening decisions"));
        assert!(!md.contains("Every AI screening decision"));
    }

    async fn plain_article(db: &ProjectDb, title: &str) -> i64 {
        db.insert_article(&NewArticle {
            title: title.into(),
            authors: None,
            year: None,
            journal: None,
            abstract_text: None,
            doi: None,
            url: None,
            source_databases: vec![],
            import_batches: vec![],
        })
        .await
        .unwrap()
    }

    fn ta<'a>(id: i64, decision: &'a str, auto: bool) -> NewScreeningDecision<'a> {
        NewScreeningDecision {
            article_id: id,
            stage: "title_abstract",
            mode: "screening",
            decision,
            criterion_number: None,
            rationale: "why",
            confidence: Some(if auto { 0.95 } else { 0.5 }),
            model_name: Some("m"),
            ai_call_id: None,
            auto_applied: auto,
            run_id: "r",
        }
    }

    fn screening_row<'a>(md: &'a str, title: &str) -> &'a str {
        md.lines()
            .find(|l| l.starts_with(&format!("| {title} |")) && (l.contains("| Included |") || l.contains("| Excluded |")))
            .unwrap()
    }

    #[tokio::test]
    async fn decided_by_follows_the_log_not_ai_rationale() {
        let (db, _d) = test_db().await;
        // AI unsure, then the researcher excluded it.
        let a = plain_article(&db, "Unsure then human").await;
        db.save_ai_screening_decision(&ta(a, "uncertain", false)).await.unwrap();
        db.apply_ai_outcome(a, "uncertain", None, "why", 0.5).await.unwrap();
        db.apply_exclusion(a, 4, Some("Wrong population")).await.unwrap();
        // AI excluded automatically and nobody changed it.
        let b = plain_article(&db, "Auto excluded").await;
        db.save_ai_screening_decision(&ta(b, "exclude", true)).await.unwrap();
        db.apply_ai_outcome(b, "exclude", Some(8), "why", 0.95).await.unwrap();
        // AI included automatically, researcher overrode to exclude.
        let c = plain_article(&db, "Overridden include").await;
        db.save_ai_screening_decision(&ta(c, "include", true)).await.unwrap();
        db.apply_ai_outcome(c, "include", None, "why", 0.95).await.unwrap();
        db.apply_exclusion(c, 4, None).await.unwrap();
        db.mark_ai_decision_overridden(c, "title_abstract", "exclude").await.unwrap();

        let md = generate_decision_report(&db).await.unwrap();
        assert!(screening_row(&md, "Unsure then human").contains("| Human |"), "{md}");
        assert!(screening_row(&md, "Auto excluded").contains("| AI |"));
        assert!(screening_row(&md, "Overridden include").contains("| Human |"));
    }

    #[tokio::test]
    async fn report_counts_decisions_whose_status_cannot_be_determined() {
        let (db, _d) = test_db().await;
        // Legacy: AI-excluded before the log existed (no log row at all).
        let a = plain_article(&db, "Legacy exclusion").await;
        db.apply_ai_outcome(a, "exclude", Some(8), "old reason", 0.95).await.unwrap();
        // Logged as unsure, then decided outside the log.
        let b = plain_article(&db, "Decided elsewhere").await;
        db.save_ai_screening_decision(&ta(b, "uncertain", false)).await.unwrap();
        sqlx::query("UPDATE article SET status='excluded' WHERE id=?").bind(b).execute(&db.pool).await.unwrap();
        // Error rows never count.
        let c = plain_article(&db, "Errored").await;
        db.save_ai_screening_decision(&ta(c, "error", false)).await.unwrap();
        // Legacy AI unsure that is still undecided does not count (no decision was applied).
        let d = plain_article(&db, "Still open").await;
        db.apply_ai_outcome(d, "uncertain", None, "hmm", 0.4).await.unwrap();

        let md = generate_decision_report(&db).await.unwrap();
        assert!(md.contains(
            "2 earlier AI screening decisions were recorded before this log existed, or were decided outside it; their review status can't be determined."
        ), "{md}");
        assert!(!md.contains("Every AI screening decision"));
    }

    #[tokio::test]
    async fn legacy_note_shown_even_without_log_rows() {
        let (db, _d) = test_db().await;
        let a = plain_article(&db, "Legacy only").await;
        db.apply_ai_outcome(a, "include", None, "old", 0.95).await.unwrap();
        let md = generate_decision_report(&db).await.unwrap();
        assert!(md.contains("1 earlier AI screening decision was recorded before this log existed, or was decided outside it; its review status can't be determined."), "{md}");
    }
}

#[cfg(test)]
mod setup_assistant_report_tests {
    use super::generate_decision_report;
    use crate::db::{test_db, DecisionInput};
    use crate::setup_assistant::{
        outcome_entry, questions_entry, suggestion_entry, KEY_OUTCOME, KEY_QUESTIONS, KEY_SUGGESTION,
    };
    use crate::suggest::{QAndA, StepContextField, StepQuestion};

    #[tokio::test]
    async fn report_lists_each_assistant_use_with_answers_suggestion_model_and_outcome() {
        let (db, _d) = test_db().await;
        let ctx = vec![StepContextField { label: "Title".into(), value: "Biases in AI".into() }];
        let qs = vec![StepQuestion { q: "Which level?".into(), options: vec!["UG".into(), "PG".into()] }];
        let ans = vec![QAndA { q: "Which level?".into(), a: "PG".into() }];

        // Use 1 (domain): questions answered, suggestion used then undone.
        let q = questions_entry("c1", "domain", "AI tools in class", &ctx, Ok(&qs), "qwen3-8b");
        db.append_decision_log_entry("onboarding", Some("domain"), KEY_QUESTIONS, &q, None, "ai").await.unwrap();
        let sgg = suggestion_entry("c1", "domain", "AI tools in class", &ctx, &ans, false, Ok("Generative AI in postgraduate teaching."), "qwen3-8b");
        db.append_decision_log_entry("onboarding", Some("domain"), KEY_SUGGESTION, &sgg, Some("Narrows the level to postgraduate."), "ai").await.unwrap();
        for (o, f) in [("used", None), ("undone", Some("AI tools in class"))] {
            let e = outcome_entry("c1", "domain", o, f).unwrap();
            db.append_decision_log_entry("onboarding", Some("domain"), KEY_OUTCOME, &e, None, "human").await.unwrap();
        }
        // Use 2 (title): skipped the questions, ignored the suggestion.
        let q = questions_entry("c2", "name", "My review", &[], Ok(&qs), "qwen3-8b");
        db.append_decision_log_entry("onboarding", Some("name"), KEY_QUESTIONS, &q, None, "ai").await.unwrap();
        let sgg = suggestion_entry("c2", "name", "My review", &[], &[], true, Ok("Biases in AI-assisted learning"), "qwen3-8b");
        db.append_decision_log_entry("onboarding", Some("name"), KEY_SUGGESTION, &sgg, Some("More specific."), "ai").await.unwrap();
        let e = outcome_entry("c2", "name", "ignored", Some("My own title")).unwrap();
        db.append_decision_log_entry("onboarding", Some("name"), KEY_OUTCOME, &e, None, "human").await.unwrap();
        // Use 3: the questions call failed.
        let q = questions_entry("c3", "rq_what", "What?", &ctx, Err("connection refused"), "qwen3-8b");
        db.append_decision_log_entry("onboarding", Some("rq_what"), KEY_QUESTIONS, &q, None, "ai").await.unwrap();
        // The researcher's final answer stays a separate entry.
        db.apply_decision(DecisionInput {
            stage: "onboarding".into(),
            sub_stage: None,
            decision_key: "project.name".into(),
            value_json: "\"My own title\"".into(),
            rationale: None,
        })
        .await
        .unwrap();

        let md = generate_decision_report(&db).await.unwrap();
        let s1 = &md[md.find("## 1. Configuration decisions").unwrap()..md.find("## 1a. Setup assistant").unwrap()];
        let s1a = &md[md.find("## 1a. Setup assistant").unwrap()..md.find("## 2. Screening decisions").unwrap()];

        // §1 keeps only the researcher's decisions.
        assert!(s1.contains("My own title"));
        assert!(!s1.contains("assistant"));
        assert!(!s1.contains("superseded"), "assistant entries must not count as revisions");

        // §1a: one block per use, in order, with every recorded part.
        assert!(s1a.find("**Domain").unwrap() < s1a.find("**Title").unwrap());
        assert!(s1a.contains("text in the field when asking: \u{201c}AI tools in class\u{201d}"));
        assert!(s1a.contains("context: Title: \u{201c}Biases in AI\u{201d}"));
        assert!(s1a.contains("AI \u{2014} clarifying questions (qwen3-8b): \u{201c}Which level?\u{201d} (options: UG / PG)"));
        assert!(s1a.contains("Researcher \u{2014} answers: \u{201c}Which level?\u{201d} \u{2192} \u{201c}PG\u{201d}"));
        assert!(s1a.contains("AI \u{2014} suggestion (qwen3-8b): \u{201c}Generative AI in postgraduate teaching.\u{201d}"));
        assert!(s1a.contains("AI \u{2014} reason: \u{201c}Narrows the level to postgraduate.\u{201d}"));
        assert!(s1a.contains("Used the suggestion \u{2192} Undid the suggestion"));
        assert!(s1a.contains("skipped the questions (\u{201c}Skip \u{2014} just suggest\u{201d})"));
        assert!(s1a.contains("Ignored the suggestion and kept their own answer (answer at that point: \u{201c}My own title\u{201d})"));
        assert!(s1a.contains("Earlier setup answers sent as context: none"));
        assert!(s1a.contains("clarifying questions failed (qwen3-8b): \u{201c}connection refused\u{201d}"));
        assert!(s1a.contains("next action: not recorded"));
    }

    #[tokio::test]
    async fn assistant_entries_are_marked_ai_and_final_answer_human() {
        let (db, _d) = test_db().await;
        let q = questions_entry("c1", "name", "x", &[], Ok(&[]), "m");
        db.append_decision_log_entry("onboarding", Some("name"), KEY_QUESTIONS, &q, None, "ai").await.unwrap();
        db.apply_decision(DecisionInput {
            stage: "onboarding".into(),
            sub_stage: None,
            decision_key: "project.name".into(),
            value_json: "\"x\"".into(),
            rationale: None,
        })
        .await
        .unwrap();
        let log = db.list_decision_log().await.unwrap();
        let actor = |k: &str| log.iter().find(|d| d.decision_key == k).unwrap().actor.clone();
        assert_eq!(actor(KEY_QUESTIONS), "ai");
        assert_eq!(actor("project.name"), "human");
        // Appending never supersedes: both entries stay effective.
        assert!(log.iter().all(|d| d.superseded_by.is_none()));
    }

    #[tokio::test]
    async fn unused_assistant_says_so() {
        let (db, _d) = test_db().await;
        let md = generate_decision_report(&db).await.unwrap();
        assert!(md.contains("## 1a. Setup assistant\n\n_The setup assistant was not used._"));
    }
}
