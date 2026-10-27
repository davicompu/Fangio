# Reeves in pictures

A walk through one real review in Reeves, for readers who would rather see the
app than install it. The project is an ongoing systematic review, *Biases as
Result of AI in Business Education*: 766 records imported from Web of Science
and EBSCO Business Source Complete, screened, coded and explored with local AI
models. The screenshots were taken with Reeves 0.4.3 on macOS, with LM Studio
running the models on the same computer.

The review is still in Stage 2, so Stage 3 (synthesis, research agenda, draft
and export) isn't pictured yet.

Back to the [README](../README.md).

---

## 1. Starting a review

![The Reeves welcome screen](screenshots/01-welcome.png)

A review is a folder. Reeves keeps everything for it in that folder: the
database, PDFs, drafts, exports and backups. The welcome screen says plainly
which features use the internet.

![Project overview with research questions](screenshots/02-project-overview.png)

The project overview opens with the research questions set during onboarding:
what is known, how it is known, and where the field should go next. Every
setup answer, and every use of the setup assistant, is recorded in the
project's decision log.

![The three SPAR-4-SLR stages](screenshots/03-pipeline.png)

The work follows the three stages of the SPAR-4-SLR protocol (Paul et al.,
2021): **Assembling**, **Arranging** and **Assessing**. A stage opens when the
one before it is done.

## 2. Assembling the literature

![Search strategy: databases, period and eligibility criteria](screenshots/04-search-strategy.png)

The search strategy records which databases are searched, the publication
period, and the inclusion and exclusion criteria.

![The Boolean query with the Advise button](screenshots/05-boolean-query.png)

The Boolean query is the string pasted into each database's advanced search.
**Advise** asks the query agent to critique and improve it. Its suggestion and
reason are logged, together with whether the researcher accepted, undid or
dismissed it.

![The imported records](screenshots/06-imported-records.png)

Records imported from each database (RIS, BibTeX, Web of Science and Scopus
exports) land in one library, here 766 records. Duplicates across databases
are found automatically (70 in this review) and kept out of screening.

![The four-condition checkpoint](screenshots/07-checkpoint.png)

Before any coding, SPAR-4-SLR asks four questions: is there enough literature,
is there no recent review, is there no high-quality review, and is there a gap
to fill? Reeves checks the first one itself and asks the researcher to confirm
the rest.

## 3. Arranging: screening and coding

![Framework dimensions and codes](screenshots/08-framework-dimensions.png)

Coding follows the review's organising framework, here TCM (Theories, Contexts,
Methods). Each dimension shows its starter codes and every code applied so far.

![Purification: exclusion criteria and journal quality](screenshots/09-purification.png)

Purification applies the exclusion criteria that can be checked from the data,
such as the search period and journal quality ratings (ABS, ABDC, impact factor),
before reading begins.

![Title and abstract screening with an AI decision](screenshots/10-title-abstract-screening.png)

Title and abstract screening shows five articles at a time. An AI assistant can
screen them first. Each AI decision shows what it decided, its reason and its
confidence, and a status: *Applied automatically*, *Opened, left unchanged*,
*Overridden* or *Pending*. Only decisions above the confidence threshold are
applied without the researcher.

![The detail of one logged AI decision](screenshots/11-ai-decision-detail.png)

Opening an AI decision shows the model that made it, its confidence, the prompt
fingerprint and temperature where recorded, and its reasoning, with key evidence
from the abstract highlighted. Each opening is logged, as "opened", not "read":
Reeves can tell that a decision was on screen, not that it was assessed.

![Full-text screening](screenshots/12-full-text-screening.png)

Full-text screening is PRISMA's second pass. Only articles kept here go on to
coding, and every exclusion needs a reason. **Find PDFs** looks for open-access
copies of articles without one.

![The coding view](screenshots/13-coding.png)

Coding works article by article. Inter-rater agreement between the two AI
coding agents is shown on the right, and filters find uncoded articles, articles
without a PDF, or ones where the agents disagreed.

![One coded article](screenshots/14-coded-article.png)

Each coded article shows its codes by dimension, with its PDF one click away.

![AI coding suggestions waiting for review](screenshots/15-ai-coding-suggestions.png)

When the two coding agents disagree, their suggestions wait for the researcher.
Each is labelled *One agent only* or *Agreed by both agents*, with the model
and the agent's reason; newer runs also show the agent's confidence (the
model's own estimate, not a measured probability). Nothing is added until the researcher accepts it, and every accept
and reject is logged.

![The live PRISMA 2020 flow diagram](screenshots/16-prisma-flow.png)

The PRISMA 2020 flow diagram (Page et al., 2021) updates itself from the
project's decisions and exports as SVG or PNG for the paper.

## 4. Asking the literature

These tools help the researcher explore the corpus. They don't change review
decisions and aren't part of the decision log.

![A cited answer from the corpus](screenshots/17-ask-the-corpus.png)

**Chat** answers questions using only the included articles, citing each claim
by article and listing the documents it drew on.

![AI-enhanced Library search](screenshots/18-ai-enhanced-search.png)

The Library has two kinds of search. *Traditional search* matches exact words.
*AI-enhanced search* ranks every paper by meaning across its title, abstract and
PDF text, and shows the passage that matched best. Here "cognitive offloading"
finds papers that discuss the idea without using the phrase in their title.

![The glossary with suggested merges](screenshots/19-glossary-merges.png)

The **glossary** collects key terms from the papers. Terms that mean the same
thing, like "Large Language Models" and "Large Language Model", are suggested
for merging, and the researcher merges, undoes or rejects each pair. Each term
counts every paper that mentions it, not only those it was extracted from.

![The concept map](screenshots/20-concept-map.png)

The **concept map** links glossary terms (circles) to the articles that discuss
them (squares).

![Three-way concept overlap](screenshots/21-concept-overlap.png)

**Concept overlap** compares two or three concepts. Here 18 articles discuss
academic integrity, AI literacy and assessment together.

![Clusters of related articles](screenshots/22-clusters.png)

**Clusters** groups articles by similarity of their full text. The number of
clusters can be chosen, and the chat model writes the labels.

## 5. Settings and transparency

![Model server settings](screenshots/23-settings-models.png)

Reeves works with any OpenAI-compatible model server. It's tested with LM Studio
on the same computer, so article text stays on that machine. The
second endpoint's address is blurred in this image.

![Thresholds, temperatures and the optional seed](screenshots/24-settings-reproducibility.png)

The confidence threshold for automatic AI decisions, the sampling temperatures
and an optional seed are all settings. Each AI decision records the model,
temperature and prompt fingerprint actually used, so others can re-run the
analysis. Model outputs can still vary between runs.

![Editable agent prompts](screenshots/25-settings-agent-prompts.png)

The instructions given to the screening and coding agents, and to the merging,
consolidating and synthesis agents, can be read and edited, with a one-click
reset to the defaults. The output format stays fixed so edits can't break the
app.

![What goes over the network](screenshots/26-settings-privacy.png)

Settings list everything that leaves the computer: DOI lookups, open-access PDF
searches, pages opened in the database browser, and whatever is sent to the
model server. Usage statistics stay in the project.

---

## The decision report

Everything above feeds the **decision report** exported in Stage 3. It records
who made each decision, the researcher or an AI agent, and for AI decisions the
model, prompt fingerprint and temperature. An excerpt from a test run on a copy
of this project:

```text
## 6. AI agents

### Query agent (search strategy)

- AI — 2026-10-27 07:47 UTC (google/gemma-4-26b-a4b, temperature 0.3, prompt 4bfbab8d783b)
  - Suggestion: "(("artificial intelligence" OR "AI" OR … OR "LLM*" OR "large language model*" …"
  - Reason: "I expanded the AI concept to include 'LLM*' and 'algorithmic grading', broadened
    the educational context with 'postsecondary' and 'vocational education' …"
  - Researcher — 2026-10-27 07:48 UTC: accepted it as the query "…"
  - Researcher — 2026-10-27 07:49 UTC: undid it
```

## References

- Page, M. J., et al. (2021). The PRISMA 2020 statement: an updated guideline for
  reporting systematic reviews. *BMJ*, 372, n71.
- Paul, J., Lim, W. M., O'Cass, A., Hao, A. W., & Bresciani, S. (2021).
  Scientific procedures and rationales for systematic literature reviews
  (SPAR-4-SLR). *International Journal of Consumer Studies*, 45(4), O1–O16.
