# Technical writing

**Write this 80% of the way to ASD-STE100.**

Use this direction for specifications, runbooks, READMEs, design notes, release notes, verification records, OpenAPI descriptions, and code comments.
The 80% target means practical clarity. It is not a measured compliance score.
[ASD-STE100](https://www.asd-ste100.org/) defines writing rules and a controlled dictionary. This project uses its clarity principles without claiming formal compliance.

## Write clear statements

- Use short sentences. Give each sentence one main idea.
- Name the actor when it matters: the caller, API process, supervisor, or worker.
- Use active verbs. Write "The supervisor waits for the child to exit."
- Use one term for each concept. Define a necessary technical term on first use.
- State the condition before the required action.
- Give limits with their units and scope. Distinguish each output from the sum of all outputs.
- Use complete sentences. Do not remove articles or join words to make text shorter.
- Use these clarity principles in Korean text too. Keep the document's intended language.

## Write procedures people can follow

State the prerequisites before the steps.
Give one action per numbered step when order matters.
Put commands in code blocks. Explain where to run them and what result to check.
Name the setting, file, endpoint, or process that a reader must change.
Explain what a failed check means and which action follows it.

## Preserve the contract and the evidence

Keep identifiers, commands, paths, error messages, version tags, hashes, and measured values exact.
Preserve conditions such as "before headers," "per request," and "after the child exits."
Keep units such as MiB, bytes, CPU seconds, and wall time distinct.
State which system owns an action or a resource.
Keep historical results tied to their original date, version, and environment.

Edit generated API descriptions in `src/contracts.rs`, then run `just contracts`.
For code comments, explain the reason or the invariant that a reader cannot see from the code alone.
Preserve native pointer ownership, allocation limits, cancellation order, and process cleanup conditions.

## Review an edit

Read the text as someone who has not seen the implementation.
Check that the reader can identify the input, action, result, owner, and limit.
Compare commands, contract values, and measurement tables with the source before accepting the edit.
Report the checks that ran. Keep local tests, container tests, remote CI, publication, and deployment as separate results.
