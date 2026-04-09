//! Framework dimension lookups.
//!
//! Each SPAR-4-SLR organising framework has a fixed set of dimensions.
//! These drive the coding UI: every article gets coded against every
//! dimension of the project's chosen framework.

pub fn dimensions_for(framework: &str) -> Vec<&'static str> {
    match framework {
        "ADO" => vec!["Antecedents", "Decisions", "Outcomes"],
        "TCM" => vec!["Theories", "Contexts", "Methods"],
        "TCCM" => vec!["Theories", "Constructs", "Characteristics", "Methods"],
        "5W1H" => vec!["What", "Why", "Where", "When", "Who", "How"],
        "ADO-TCM" => vec![
            "Antecedents",
            "Decisions",
            "Outcomes",
            "Theories",
            "Contexts",
            "Methods",
        ],
        _ => vec![],
    }
}

pub fn description_for(framework: &str, dimension: &str) -> &'static str {
    match (framework, dimension) {
        (_, "Antecedents") => "Factors that influence or precede the central behaviour or decision.",
        (_, "Decisions") => "The core behaviours, choices, or actions the article studies.",
        (_, "Outcomes") => "Consequences, results, or effects that follow from the decisions.",
        (_, "Theories") => "Theoretical frameworks the article draws on or contributes to.",
        (_, "Contexts") => "Settings, samples, geographies, or industries the article studies.",
        (_, "Methods") => "Research methods, designs, and analytical approaches used.",
        (_, "Constructs") => "Specific variables, constructs, or concepts measured.",
        (_, "Characteristics") => "Sample, study, or design characteristics.",
        (_, "What") => "The phenomenon under study.",
        (_, "Why") => "The motivation or rationale for studying it.",
        (_, "Where") => "The geographic, industrial, or institutional setting.",
        (_, "When") => "The temporal context or period studied.",
        (_, "Who") => "The population, sample, or actors involved.",
        (_, "How") => "The methods and approaches used to study it.",
        _ => "",
    }
}
