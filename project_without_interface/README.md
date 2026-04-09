# SPAR-4-SLR Research Agent

## Overview

This agent guides researchers through conducting Systematic Literature Reviews (SLRs) following the **Scientific Procedures and Rationales for Systematic Literature Reviews (SPAR-4-SLR)** protocol developed by Paul, Lim, O'Cass, Hao, and Bresciani (2021).

The agent operationalizes the SPAR-4-SLR protocol by walking researchers through key decisions in a structured, conversational way — asking the right questions at each stage, recommending best practices, and justifying those recommendations based on the published protocol.

Where appropriate, the agent may also generate a **PRISMA 2020** flow diagram and checklist to support transparent reporting of the search and screening process, consistent with the updated guidelines of Page et al. (2021).

---

## Getting Started

**To run the SLR Agent, load the Orchestrator.**

Open [`agents/orchestrator_agent.md`](agents/orchestrator_agent.md) and use its contents as the system prompt for your Claude session. The Orchestrator is the single entry point — it handles onboarding, routes you to the correct sub-agent at each stage, and manages resumption when you return to a session.

> Do not load the sub-agent prompts (`assembling_agent.md`, `arranging_agent.md`, `assessing_agent.md`) directly. They are invoked by the Orchestrator, not by the researcher.

---

## Available Tools

The following skills/tools are available to extend the agent's capabilities:

| Tool | Description | Requirement |
|-------|-------------|-------------|
| [playwright-cli](skills/playwright_cli/SKILL.md) | Automates browser interactions to navigate and search academic databases (e.g., WOS, Scopus) directly from your session | Requires [Playwright](https://playwright.dev/) installed on your machine (`npm install -g playwright`) |

---

## References

> Page, M. J., McKenzie, J. E., Bossuyt, P. M., Boutron, I., Hoffmann, T. C., Mulrow, C. D., Shamseer, L., Tetzlaff, J. M., Akl, E. A., Brennan, S. E., Chou, R., Glanville, J., Grimshaw, J. M., Hróbjartsson, A., Lalu, M. M., Li, T., Loder, E. W., Mayo-Wilson, E., McDonald, S., … Moher, D. (2021). The PRISMA 2020 statement: An updated guideline for reporting systematic reviews. *BMJ*, 372, n71. https://doi.org/10.1136/bmj.n71

> Paul, J., Lim, W. M., O'Cass, A., Hao, A. W., & Bresciani, S. (2021). Scientific procedures and rationales for systematic literature reviews (SPAR-4-SLR). *International Journal of Consumer Studies*, 45, O1–O16. https://doi.org/10.1111/ijcs.12695

---

## SLR Types Supported

The agent supports all five main SLR types identified in the SPAR-4-SLR protocol:

| # | SLR Type | Description |
|---|----------|-------------|
| 1 | **Domain-based** | Concentrates on the development of an area, outlet, or topic |
| 2 | **Theory-based** | Examines the development of a specific theory in a domain |
| 3 | **Method-based** | Explores the development of a specific method in a domain |
| 4 | **Meta-analytical** | Focuses on statistical assessments of prior quantitative research |
| 5 | **Meta-systematic** | Consolidates existing systematic reviews in a domain |

> **Default recommendation:** If the user is unsure, the agent recommends a **Domain-based / Framework-based** review, consistent with the SPAR-4-SLR authors' own guidance that framework-based reviews deliver the "highest level of clarity and coverage" and are "often more useful and impactful than all other types."

---

## Domain-Based Review Sub-types

When a user selects or is guided toward a domain-based review, the agent further clarifies the sub-type:

| Sub-type | Focus | Best When |
|----------|-------|-----------|
| **Structured theme-based** | Development of themes (theories, constructs, contexts, methods) | Mapping a broad topic area |
| **Framework-based** | Uses established frameworks to organize the review | Seeking maximum clarity, depth, and impact ⭐ *Recommended* |
| **Bibliometric** | Statistical and trend analysis of publications | Mapping output, citations, collaborations |
| **Hybrid** | Combines two or more sub-forms | When a single sub-form is insufficient |
| **Conceptual** | Proposes new theories, hypotheses, propositions | Theory-building goals |

---

## Framework Comparison for Framework-Based Reviews

When a researcher chooses (or is guided toward) a **framework-based domain review**, the agent asks questions to identify the most suitable organizing framework. The four recommended frameworks are compared below.

### Comparative Table: ADO vs TCM vs TCCM vs 5W1H

| Dimension | **ADO** | **TCM** | **TCCM** | **5W1H** |
|-----------|---------|---------|----------|----------|
| **Full Name** | Antecedents, Decisions, Outcomes | Theories, Contexts, Methods | Theories, Constructs, Characteristics, Methods | What, Why, Where, When, Who, How |
| **Primary Focus** | Mapping causal and relational flows in a domain | Mapping the theoretical, contextual, and methodological landscape | Comprehensive mapping of constructs and characteristics alongside theories and methods | Interrogative exploration of a domain across six dimensions |
| **Core Question** | What drives decisions, and what results from them? | What theories, contexts, and methods define this domain? | What theories, constructs, characteristics, and methods define this domain? | What do we know, why, where, when, who studied it, and how? |
| **Key Strengths** | - Clear causal structure<br>- Intuitive antecedent-outcome logic<br>- Strong for empirically rich domains<br>- Easy to visualize as a model | - Comprehensive theoretical mapping<br>- Strong for identifying research gaps<br>- Works well across disciplines<br>- Good for methodological critique | - Most granular of the four<br>- Captures constructs AND characteristics<br>- Ideal for mature, dense domains<br>- Useful for guiding future empirical research | - Most flexible and accessible<br>- Covers broadest range of questions<br>- Good for interdisciplinary topics<br>- Easy to communicate to diverse audiences |
| **Coding Structure** | Antecedents → Decisions → Outcomes (often in matrix form) | Theory used / Context / Method (one row per article) | Theory / Constructs / Characteristics / Method (one row per article) | Six dimensions coded per article or per theme |
| **Ideal Scenario** | - Behavioral or managerial research<br>- Decision-making processes<br>- Consumer behavior topics<br>- When cause-effect logic is central | - Emerging or growing domains<br>- When theory gaps are a concern<br>- Cross-disciplinary topics<br>- When methods diversity needs mapping | - Mature, well-developed domains<br>- When construct clarity is needed<br>- Empirical research planning<br>- When characteristics (e.g., sample, country) matter | - Broad or exploratory domains<br>- Introductory or scoping reviews<br>- When communicating to policy or practice audiences<br>- When no single lens is sufficient |
| **Can Be Combined With** | TCM (→ integrated ADO-TCM) | ADO (→ integrated ADO-TCM) | Standalone or with ADO elements | Any of the above |
| **Representative Exemplars** | Paul & Benito (2018); Lim, Yap et al. (2021) | Paul et al. (2017); Khatoon & Rehman (2021) | Paul & Rosado-Serrano (2019); Billore & Anisimova (2021) | Lim (2020); Billore & Anisimova (2021) |
| **Best Fit Research Questions** | "What leads to X, and what are the consequences?" | "What theories and methods have been used to study X?" | "What constructs and characteristics define X research?" | "What do we know about X, and where should it go?" |

> **Note:** The **integrated ADO-TCM framework** (Lim, Yap et al., 2021) is also recommended when researchers want to capture both the relational flow (ADO) and the theoretical/methodological landscape (TCM) in a single review. This is considered the most comprehensive option for mature domains.

---

## The SPAR-4-SLR Protocol: Three Stages

The agent walks researchers through all three stages and six sub-stages of the protocol:

```
ASSEMBLING
├── 1a. Identification  →  Domain, Research Questions, Source Type, Source Quality (incl. Language, Deferred Filters)
└── 1b. Acquisition     →  Institutional Access, Search Mechanism, Search Period, Search Keywords

ARRANGING
├── 2a. Organization    →  Organizing Codes, Organizing Frameworks
└── 2b. Purification    →  Article Inclusion/Exclusion Criteria

ASSESSING
├── 3a. Evaluation      →  Analysis Method, Agenda Proposal Method
└── 3b. Reporting       →  Reporting Conventions, Limitations, Support Sources
```

---

## Agent Design Principles

The agent is designed around the following principles derived from SPAR-4-SLR:

1. **Ask before assuming** — The agent asks the user about their review type, domain, and goals before making recommendations.
2. **One question at a time** — The agent presents exactly one question, decision, or item per turn. It waits for the researcher's response before moving to the next topic. This prevents information overload and keeps the conversation focused and manageable.
3. **Recommend with rationale** — Every recommendation the agent makes is accompanied by a justification grounded in the SPAR-4-SLR protocol.
4. **Default to best practice** — When users are uncertain, the agent defaults to framework-based domain reviews and guides toward ADO-TCM or TCCM for mature domains.
5. **Discourage bad practices** — The agent gently discourages SLRs conducted for the wrong reasons (e.g., CV building, avoiding fieldwork) as described in the protocol.
6. **Transparency first** — The agent ensures users document every decision with a rationale, supporting reproducibility.
7. **Human judgment preserved** — The agent guides but does not replace researcher expertise. It flags decisions that require disciplinary knowledge.

---

## When Should an SLR Be Written? (Agent Logic)

The agent reviews four conditions as a **checkpoint between Stage 1 and Stage 2**, after the search has been executed and articles_retrieved.csv is available. These conditions require evidence from the literature and cannot be assessed during onboarding. Based on SPAR-4-SLR Section 4:

| Condition | Threshold | Agent Action if Not Met |
|-----------|-----------|------------------------|
| Sufficient body of work exists | ≥ 40 articles | Suggest writing a position paper instead |
| No recent SLR exists in the domain | Within last 5 years | Warn user; ask them to justify novelty |
| No high-quality SLR exists | CABS 4*/4/3, ABDC A*/A, WOS/Scopus Q1/Q2 | Flag opportunity; encourage high-quality outlet targeting |
| Existing SLRs have gaps/shortchanges | Identifiable gaps | Require user to articulate specific gaps |

---

## Source Quality Reference

| Quality List | Tiers Recommended | Notes |
|-------------|-------------------|-------|
| **WOS (SSCI/SCI)** | Impact Factor ≥ 1 | Fewer journals, stricter; good for established domains |
| **Scopus** | Q1 / Q2 | Broader coverage; good for emerging domains |
| **CABS AJG** | 4* / 4 / 3 | Business discipline; updated every few years |
| **ABDC JQL** | A* / A | Business discipline; broader than CABS |
| **Impact Factor** | IF ≥ 1 / ≥ 2 / ≥ 5 | Alternative or fallback when formal lists don't cover the domain; **deferred filter** — cannot be applied at search time, enforced during Stage 2 purification |

> The agent recommends choosing **only one** journal quality list to avoid reconciliation issues across lists.
>
> **Language restriction:** The agent suggests English-only articles as the default. Researchers may include other languages with documented rationale.

---

## Multi-Agent Architecture

The agent system is organized as an **orchestrator + three sub-agents**, mapping directly onto the three stages of the SPAR-4-SLR protocol.

```
User
  ↕
Orchestrator Agent
  ├── Assembling Agent  (Stage 1)
  ├── Arranging Agent   (Stage 2)
  └── Assessing Agent   (Stage 3)
```

### Agent Responsibilities

| Agent | Responsibilities |
|-------|-----------------|
| **Orchestrator** | Onboarding, SLR type & framework selection, resumption logic, stage routing, inter-agent communication, sequential enforcement |
| **Assembling Agent** | Identification and acquisition sub-stages; writes `assembling_report.md` |
| **Arranging Agent** | Organization and purification sub-stages; writes `arranging_report.md` |
| **Assessing Agent** | Evaluation and reporting sub-stages; writes `assessing_report.md`; generates `drafts/slr_draft.md` |

### Design Principles

**1. Orchestrator owns onboarding and resumption**
The orchestrator handles all pre-stage work — including the initial conversation about SLR type, framework selection, domain definition, and the "when should this SLR be written" checklist — before routing to any sub-agent. When a researcher resumes work, the orchestrator reads all three reports to determine where they left off and which agent to call next.

**2. Each agent owns its report**
Each sub-agent writes and maintains its own Markdown report, which serves as the reproducibility record for that stage. This directly reflects the SPAR-4-SLR requirement that *"the process must be completely and transparently declared in order for the outcome to be reproducible."* The orchestrator reads all three reports to determine system state.

**3. Orchestrator manages all communication**
Sub-agents do not communicate with each other directly. All routing, handoffs, and state checks flow through the orchestrator. This enforces the sequential stage order that SPAR-4-SLR explicitly requires, reflecting the protocol's warning against modifying its arrangements and conventions.

**4. Two types of routing logic**
The orchestrator supports two routing modes:

- **Forward progression** — moves the researcher from Assembling → Arranging → Assessing when a stage is marked complete
- **Intra-stage iteration** — allows the researcher to revisit and revise decisions *within* a stage without invalidating the stage entirely; however, when upstream changes affect downstream stages, the orchestrator flags the impact and prompts the researcher to review affected reports before proceeding

---

### Report Schema

Each sub-agent maintains a Markdown report with a standardized structure. This keeps the schema machine-readable for the orchestrator while remaining human-readable for researchers.

Report templates are stored in `agent_reports/report_templates/` and are read by each agent only when creating a new report for the first time. On subsequent sessions, agents read the actual report file instead.

| Report | Template |
|--------|----------|
| `agent_reports/assembling_report.md` | [`agent_reports/report_templates/assembling_report_template.md`](agent_reports/report_templates/assembling_report_template.md) |
| `agent_reports/arranging_report.md` | [`agent_reports/report_templates/arranging_report_template.md`](agent_reports/report_templates/arranging_report_template.md) |
| `agent_reports/assessing_report.md` | [`agent_reports/report_templates/assessing_report_template.md`](agent_reports/report_templates/assessing_report_template.md) |

---

## Project Structure (Planned)

```
spar4slr-agent/
├── README.md                             ← This file
├── agents/
│   ├── orchestrator_agent.md     ← Orchestrator agent system prompt
│   ├── assembling_agent.md       ← Assembling agent system prompt (Stage 1)
│   ├── arranging_agent.md        ← Arranging agent system prompt (Stage 2)
│   └── assessing_agent.md        ← Assessing agent system prompt (Stage 3)
├── agent_reports/
│   ├── assembling_report.md             ← Assembling stage progress & decisions
│   ├── arranging_report.md              ← Arranging stage progress & decisions
│   ├── assessing_report.md              ← Assessing stage progress & decisions
│   └── report_templates/
│       ├── assembling_report_template.md ← Template for new assembling reports
│       ├── arranging_report_template.md  ← Template for new arranging reports
│       └── assessing_report_template.md  ← Template for new assessing reports
├── data/
│   ├── articles_retrieved.csv            ← All articles collected by Assembling Agent (with metadata)
│   ├── articles_included.csv             ← Final article pool after purification by Arranging Agent
│   └── articles_excluded.csv             ← Excluded articles with exclusion criterion and rationale
└── drafts/
    └── slr_draft.md                      ← Draft report generated by Assessing Agent after Stage 3
```

---

## License

This project is for academic and research support purposes. The SPAR-4-SLR protocol is credited to Paul et al. (2021) and this agent is an independent implementation inspired by their work.
---

## Architectural Design Decision: Pure Stateless Communication

### Decision
All agents in this system communicate exclusively through profile 
documents and CSV files. Agents do not share or rely on conversation 
context between sessions. Every session starts fresh — the agent 
reads the relevant files to establish state before doing anything else.

This means:
- Sub-agents read state only from their report and CSV files
- Sub-agents write every decision to their report immediately
- No decision exists unless it is recorded in a file
- Conversation context is never treated as a source of truth
- The Orchestrator reads all reports and checks CSV outputs 
  to determine pipeline state at the start of every session

### Rationale
This design decision is a direct expression of the core values of 
the SPAR-4-SLR protocol:

**Transparency** — every decision made during the review is visible 
in the reports and CSV files. Nothing is hidden in 
conversation history or agent memory. A third party can read the 
files and fully reconstruct every decision made and why.

**Reproducibility** — another researcher with access to the profile 
documents and CSV files can replicate the exact same review process 
from start to finish, including every keyword, every exclusion 
criterion, and every organizing code applied.

**Resilience** — sessions can be interrupted, resumed days or weeks 
later, or handed to a different researcher without any loss of 
progress or context. The files are the system state.

**Auditability** — journal reviewers and editors can request the 
reports and CSV files as evidence that the SLR was 
conducted rigorously and systematically.

### Enabling Replication by Other Researchers
One of the most significant benefits of this stateless, file-based 
design is that it makes the entire SLR process replicable by any 
researcher, not just the original team.

In traditional SLRs, the decisions made during the review process 
are often documented partially and informally — a methods section 
describes what was done, but not always why, and rarely in enough 
detail for another researcher to replicate the process exactly. 
This is one of the most persistent criticisms of SLRs in the 
literature, and one that SPAR-4-SLR explicitly seeks to address.

This system solves that problem architecturally. Because every 
decision — every keyword chosen, every synonym considered, every 
exclusion criterion applied, every organizing code assigned — is 
written to a report or CSV file in real time, the entire 
review process becomes a fully documented artifact. A researcher 
who had no involvement in the original review could:

- Read assembling_report.md and understand exactly how the search 
  was designed, including the boolean query, the search engines used, 
  the quality thresholds applied, and the rationale for each decision
- Open articles_retrieved.csv and see every article returned by the 
  search, including which engine found it and whether it was available 
  for download
- Read arranging_report.md and understand exactly which organizing 
  codes were applied and why, and which framework guided the coding
- Open articles_excluded.csv and see every article that was removed 
  from the pool, with the specific criterion and rationale for each 
  exclusion
- Open articles_included.csv and see the exact final pool of articles 
  that entered the review, and replicate the search to verify it

This level of documentation transforms the SLR from a reported 
process into a verifiable one. It supports not just replication 
but also extension — a future researcher could take the same 
reports, adjust the search period or add new keywords, 
and produce an updated version of the review with full traceability 
back to the original.

This is precisely what Paul et al. (2021) envisioned when they 
wrote that the SPAR-4-SLR process must be "completely and 
transparently declared in order for the outcome to be reproducible." 
This system makes that declaration automatic, not optional.

### Implementation Rule
Every agent system prompt includes a **Session Protocol** section 
that instructs the agent to:
1. Read its report at the start of every session
2. Confirm the current status before taking any action
3. Write every decision to the report immediately after 
   the researcher confirms it
4. Never rely on information that is not present in the files
5. If information is missing from the files, ask the researcher 
   or escalate to the Orchestrator — never assume