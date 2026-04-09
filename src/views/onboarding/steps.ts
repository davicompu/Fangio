/**
 * Onboarding decision tree, mined from project_without_interface/agents/orchestrator_agent.md
 * and assembling_agent.md.
 *
 * Each step captures one decision. The wizard walks through them in order
 * and persists each answer to decision_log + the project row.
 */

export type StepKind =
  | "choice"          // single-select from cards
  | "text"            // free text
  | "longText"        // multi-line free text
  | "review"          // summary screen at the end
  ;

export interface ChoiceOption {
  value: string;
  label: string;
  description: string;
  recommended?: boolean;
  badge?: string;
}

export interface OnboardingStep {
  id: string;
  decisionKey: string;     // matches db.rs apply_decision known keys
  kind: StepKind;
  eyebrow: string;
  title: string;
  subtitle?: string;
  placeholder?: string;
  hint?: string;
  options?: ChoiceOption[];
  /** Inline doc snippet rendered in the WhyThisStepPanel */
  why: {
    headline: string;
    body: string;            // markdown-ish plain text, line breaks preserved
    citation?: string;
  };
  validate?: (value: string) => string | null;
}

export const SLR_TYPE_OPTIONS: ChoiceOption[] = [
  {
    value: "domain",
    label: "Domain-based",
    description:
      "Map what the field knows about a topic, journal, or area of practice. The most common SLR type — and what SPAR-4-SLR was originally written for.",
    recommended: true,
  },
  {
    value: "theory",
    label: "Theory-based",
    description:
      "Trace how a single theory has been applied, refined, or contested across the literature.",
  },
  {
    value: "method",
    label: "Method-based",
    description:
      "Survey how a particular research method (e.g., experimental designs, fsQCA) has been used and how its application has evolved.",
  },
  {
    value: "meta-analytical",
    label: "Meta-analytical",
    description:
      "Quantitatively pool effect sizes from prior studies. Requires sufficient homogeneous quantitative work.",
  },
  {
    value: "meta-systematic",
    label: "Meta-systematic",
    description:
      "A systematic review of systematic reviews — synthesise existing SLRs in an area.",
  },
];

export const FRAMEWORK_OPTIONS: ChoiceOption[] = [
  {
    value: "ADO",
    label: "ADO — Antecedents, Decisions, Outcomes",
    description:
      "Best for behavioural and decision-oriented domains. Captures what causes a behaviour, what the behaviour is, and what consequences follow.",
    recommended: true,
  },
  {
    value: "TCM",
    label: "TCM — Theories, Contexts, Methods",
    description:
      "Best for emerging domains where you want to surface theoretical and methodological gaps.",
  },
  {
    value: "TCCM",
    label: "TCCM — Theories, Constructs, Characteristics, Methods",
    description:
      "Best for mature domains where construct-level granularity matters.",
  },
  {
    value: "5W1H",
    label: "5W1H — What, Why, Where, When, Who, How",
    description:
      "Best for broad or exploratory domains where you want a journalistic, comprehensive view.",
  },
  {
    value: "ADO-TCM",
    label: "ADO-TCM (integrated)",
    description:
      "The most comprehensive option — combines ADO with TCM. Best for mature, theory-rich domains where you have time to code thoroughly.",
    badge: "Comprehensive",
  },
];

export const QUALITY_LIST_OPTIONS: ChoiceOption[] = [
  {
    value: "CABS",
    label: "CABS Academic Journal Guide (AJG)",
    description: "Standard for business and management research. Threshold typically 3 / 4 / 4*.",
    recommended: true,
  },
  {
    value: "ABDC",
    label: "ABDC Journal Quality List",
    description: "Australian standard for business research. Threshold typically A / A*.",
  },
  {
    value: "WOS-SSCI",
    label: "Web of Science SSCI",
    description: "Social sciences citation index. Threshold typically Q1 / Q2.",
  },
  {
    value: "WOS-SCI",
    label: "Web of Science SCI",
    description: "Sciences citation index. Threshold typically Q1 / Q2.",
  },
  {
    value: "Scopus",
    label: "Scopus CiteScore",
    description: "Cross-disciplinary alternative. Threshold typically Q1 / Q2.",
  },
];

export const STEPS: OnboardingStep[] = [
  {
    id: "name",
    decisionKey: "project.name",
    kind: "text",
    eyebrow: "Step 1 of 7 · Project",
    title: "What should we call this review?",
    subtitle:
      "A short working title. You can change it later — it just needs to be something you'll recognise in the recents list.",
    placeholder: "e.g., Consumer behaviour in social commerce, 2014–2025",
    why: {
      headline: "Why a working title now?",
      body:
        "A working title forces you to commit to scope. SPAR-4-SLR begins with domain definition — and the best way to test whether you've defined a domain crisply is to try to name it in one line. If you can't, the domain is still too fuzzy.",
    },
    validate: (v) => (v.trim().length < 3 ? "At least a few characters, please." : null),
  },
  {
    id: "slr_type",
    decisionKey: "project.slr_type",
    kind: "choice",
    eyebrow: "Step 2 of 7 · SLR Type",
    title: "What kind of systematic review is this?",
    subtitle:
      "SPAR-4-SLR distinguishes five types. Most reviews are domain-based — pick that if you're unsure.",
    options: SLR_TYPE_OPTIONS,
    why: {
      headline: "Why this matters",
      body:
        "Your SLR type determines everything downstream — which databases to prioritise, how to structure the search, which framework can sensibly organise the results, and which analysis methods are appropriate. Domain reviews map a field; theory reviews trace a single construct; method reviews follow a technique; meta-analyses pool numbers; meta-reviews synthesise other reviews.",
      citation: "Paul, Lim, O'Cass, Hao & Bresciani (2021). SPAR-4-SLR.",
    },
  },
  {
    id: "framework",
    decisionKey: "project.framework",
    kind: "choice",
    eyebrow: "Step 3 of 7 · Organising Framework",
    title: "Which framework will organise your findings?",
    subtitle:
      "You'll use this framework to code every included article. Don't worry about getting it perfect — you can refine the codes later.",
    options: FRAMEWORK_OPTIONS,
    why: {
      headline: "How to choose",
      body:
        "ADO is the safest default for behavioural research — it forces you to identify what causes a behaviour, what the behaviour is, and what consequences follow.\n\nTCM is best when you suspect there's a theoretical or methodological gap to surface. TCCM extends TCM with construct-level coding for mature fields. 5W1H is the broadest, best for exploratory reviews. ADO-TCM is the gold standard but takes the most coding effort.",
      citation: "Paul & Rosado-Serrano (2019); Paul & Benito (2018).",
    },
  },
  {
    id: "domain",
    decisionKey: "project.domain",
    kind: "longText",
    eyebrow: "Step 4 of 7 · Domain",
    title: "Describe the domain in your own words.",
    subtitle:
      "A few sentences. What is the area? Who's it for? What's the boundary? This becomes the anchor for every keyword and exclusion decision later.",
    placeholder:
      "e.g., How consumers behave inside social commerce platforms — environments where social media interaction and e-commerce purchasing are tightly integrated. Excludes traditional e-commerce without a social layer.",
    why: {
      headline: "Why a paragraph, not a phrase",
      body:
        "A one-line domain ('social commerce') is too thin to drive search and screening decisions. A paragraph forces you to name what's in, what's out, and what you mean by ambiguous terms. Every exclusion decision in Stage 2 will reference this paragraph — so the more concrete it is now, the easier the screening will be later.",
    },
    validate: (v) => (v.trim().length < 30 ? "A sentence or two — say what's in and what's out." : null),
  },
  {
    id: "rq_what",
    decisionKey: "project.rq_what",
    kind: "longText",
    eyebrow: "Step 5 of 7 · Research Questions (1 of 3)",
    title: "What do we know about this domain?",
    subtitle:
      "The discovery question. SPAR-4-SLR mandates three: what we know, how we know it, where the field should head. This is the first.",
    placeholder:
      "e.g., What is currently known about the antecedents, behaviours, and outcomes of consumer engagement in social commerce platforms?",
    why: {
      headline: "The 'what' question",
      body:
        "Every domain SLR answers three questions. The first describes the state of knowledge — concepts, findings, debates. Phrase it broadly enough that the entire included corpus can plausibly contribute, but specifically enough that it doesn't read like a textbook chapter.",
    },
    validate: (v) => (v.trim().length < 15 ? "Phrase it as a full question." : null),
  },
  {
    id: "rq_how",
    decisionKey: "project.rq_how",
    kind: "longText",
    eyebrow: "Step 6 of 7 · Research Questions (2 of 3)",
    title: "How do we know what we know?",
    subtitle:
      "The methods + theory question. What theoretical lenses have been used? What methods? Where are the methodological gaps?",
    placeholder:
      "e.g., What theories, contexts, and methods have been used to study consumer behaviour in social commerce, and how has methodological rigour evolved?",
    why: {
      headline: "The 'how' question",
      body:
        "This is where TCM/TCCM-style framework thinking earns its keep. The 'how' question surfaces theoretical foundations, sample contexts, study designs, measurement choices, and analytical approaches — all the things that explain why the field knows what it knows, and where it might be wrong.",
    },
    validate: (v) => (v.trim().length < 15 ? "Phrase it as a full question." : null),
  },
  {
    id: "rq_where",
    decisionKey: "project.rq_where",
    kind: "longText",
    eyebrow: "Step 7 of 7 · Research Questions (3 of 3)",
    title: "Where should this field be heading?",
    subtitle:
      "The agenda question. You don't need the answer yet — Stage 3 will help you build it. But naming the question now keeps the whole review pointed at it.",
    placeholder:
      "e.g., What gaps, contradictions, and unexplored contexts should future research on consumer behaviour in social commerce address?",
    why: {
      headline: "The 'where' question",
      body:
        "This is the question journals actually care about. Every SLR's contribution lives in the future research agenda — and naming the question now means every coding and screening decision later can be checked against it. If an article doesn't help you answer 'what's next?', it probably isn't pulling its weight in your corpus.",
    },
    validate: (v) => (v.trim().length < 15 ? "Phrase it as a full question." : null),
  },
];
