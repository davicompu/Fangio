# AI transparency requirements (2026-10-26)

Verbatim from the researcher. Items 1–6 fix existing behaviour; 7–13 add features. 7b was added mid-session.

1. Make the screening log complete. Every AI screening decision should leave a record, at both title and abstract and full text. Today some are lost: full-text includes, unsure full-text decisions, unsure results in purification mode, and model errors. Each record should keep the decision, reason, confidence and model, so the researcher can see everything the AI did. Check it works by: screening a small test project and finding a record for every article.

2. Log reverts and resets. Undoing a screening decision or resetting a full-text decision currently erases the AI's reason and confidence without a trace. Save the old decision, reason and confidence in the log first. Check it works by: undoing an AI exclusion and resetting a full-text decision, then finding both in the log.

3. Keep automatic decisions, but make them visible. Very confident AI decisions should still be applied automatically, with today's threshold. But the researcher should always see which decisions were applied this way, and the report should list them. Check it works by: confirming the same articles are applied as before, each marked as applied automatically.

4. Keep both coders' reasons for agreed codes. When both coding agents agree, Reeves keeps only the first agent's reason and model. Keep both. Also log when the researcher accepts a suggestion, as we already do for rejections. Check it works by: seeing both reasons on agreed codes, and a log entry after accepting one.

5. Rename the third coding agent to "merging agent". It only merges leftover codes that mean the same thing; it is not a tie-breaker, and real disagreements go to the researcher. Use this one name everywhere, replacing "adjudicator". Record its model and reason for each merge. Merges stay automatic and undoable, and undos are logged. Check it works by: finding no visible "adjudicator" in the app, and an undone merge in the log.

6. Fix wording that overpromises. The app and README say nothing leaves the laptop and that Reeves only connects to the local machine. Neither is true: it looks up references online and can use a remote model. Say plainly what goes out, and that usage data stays local. Remove the "coming next" list of things already built and tools we do not use. Say LM Studio and macOS are tested, and Ollama should work but is untested. Check it works by: reading the README and welcome screen and finding no claim the app cannot back up.

7. Show a review status for each AI screening decision. Each AI screening decision should show one of four statuses: applied automatically, opened and left unchanged, overridden, or pending. "Opened" means opened in the app; never say "read" or "reviewed". Show it next to each decision and in the report. Screening only, not coding. Check it works by: making one example of each status and seeing it in the app and report.

7b. Every time the researcher opens an AI screening decision to look at it, record it in the log with the date and time, the decision, and who opened it. Keep the first and the most recent time opened, and how many times it was opened. The log should say 'opened' only, never 'read' or 'reviewed', because the app can tell that a decision was loaded on screen, not that the person thought about it. Show 'Never opened' for decisions that were applied automatically and not loaded since. Check it works by: open one decision twice and leave another untouched, then see the log show two opens for the first and 'Never opened' for the second.

8. Add reminders that never block. Before moving to the next stage or exporting, if AI decisions are still applied automatically or pending, show a reminder with the count, offering "Open them now" and "Continue anyway". Record the choice. Never stop the researcher from moving on. Check it works by: the reminder appearing only when needed, and "Continue anyway" always working.

9. Show confidence for coding suggestions. Ask the coding agents for a confidence level with each suggested code and show it, noting it is the model's own estimate, not a measured probability. It should not change what gets coded. Check it works by: running AI coding and seeing a confidence next to each suggestion.

10. Show agreed codes as suggestions, with a setting. Add a setting: apply agreed codes automatically (today's behaviour, kept for existing projects) or show them as suggestions. In suggestion mode, label codes "Agreed by both agents" or "One agent only", and allow accepting all agreed codes at once. The merging agent only works on "One agent only" codes. Check it works by: no AI code being saved in suggestion mode until accepted, and the old mode giving the same results.

11. Let the researcher choose the merging agent's model, with a warning. Let the researcher pick models for the merging and consolidating agents, defaulting to the main chat model. If either matches a coding agent's model, warn that a model may favour its own wording, without blocking. Check it works by: the chosen model being used and recorded, and the warning showing only when models match.

12. Let the researcher edit the remaining agent prompts. Make the merging, consolidating and synthesis agents' instructions editable in Settings, with "Reset to default". The default is exactly today's text. Keep output format instructions fixed so edits cannot break the app. Check it works by: an edited prompt being used, and reset restoring the original.

13. Record the model, prompt version and temperature with each AI decision. For every AI screening decision, coding suggestion, merge and consolidation, save the model actually used, a fingerprint of the prompt, and the temperature, and show them in the report. Add an optional seed setting, off by default. Do not claim runs are exactly repeatable. Check it works by: seeing model, prompt fingerprint and temperature for each decision in the report.
