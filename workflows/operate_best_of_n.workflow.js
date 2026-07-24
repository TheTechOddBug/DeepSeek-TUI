/**
 * Operate starter — best-of-N implementers in worktrees, then reviewer.
 *
 * Spawns N worktree implementers with the same brief, then a read-only
 * reviewer that picks a winner. The parent must apply the winner only after
 * PASS evidence (skill: best-of-n).
 *
 * Run: /workflow run workflows/operate_best_of_n.workflow.js
 * Args: { brief, n?, targetFiles?, writeRoots? }
 */
export default async function (args) {
  const brief =
    args?.brief ??
    args?.task ??
    "Propose and implement the smallest correct fix for the open failure.";
  const n = Math.min(4, Math.max(2, Number(args?.n ?? 3) || 3));
  const exactFiles = Array.isArray(args?.targetFiles) ? args.targetFiles : [];
  const writeRoots = Array.isArray(args?.writeRoots) ? args.writeRoots : [];

  phase("Candidates");
  const candidateFns = [];
  for (let i = 1; i <= n; i++) {
    const index = i;
    candidateFns.push(() =>
      task({
        description: `Best-of-N candidate ${index}/${n}`,
        label: `candidate_${index}`,
        type: "implementer",
        worktree: true,
        writeAuthority: "worktree_write",
        ...(exactFiles.length ? { exactFiles } : {}),
        ...(writeRoots.length ? { writeRoots } : {}),
        coordinationContracts: [`best-of-n-candidate-${index}`],
        dependencies: [
          "Do not share other candidates' answers.",
          "Parent checkout must remain unchanged until apply.",
        ],
        acceptance: [
          "Return VERDICT PASS/FAIL with command evidence.",
          "List every modified path with a one-line why.",
        ],
        prompt: [
          `You are candidate ${index} of ${n} in a best-of-N tournament.`,
          "Implement the brief below in this isolated worktree only.",
          "Run relevant checks. End with VERDICT: PASS|FAIL, COMMANDS, EVIDENCE.",
          "Do not push. Do not merge. Do not touch the parent checkout.",
          "",
          "BRIEF:",
          String(brief),
        ].join("\n"),
      })
    );
  }
  const candidates = await parallel(candidateFns);

  phase("Review");
  const review = await task({
    description:
      "Score candidates against one rubric; name a winner only with PASS evidence.",
    label: "reviewer",
    type: "review",
    worktree: false,
    prompt: [
      "You are the tournament judge. Score every candidate against: correctness,",
      "fit to the brief, simplicity, risk, and verification evidence.",
      "Reject candidates without PASS command evidence for code work.",
      "Name exactly one winner (or NONE if all fail), with decisive reasons.",
      "Do not merge or apply changes. Do not invent missing evidence.",
      "",
      "candidates:",
      String(JSON.stringify(candidates, null, 2) ?? "(missing)"),
    ].join("\n"),
  });

  return {
    scenario: "operate-best-of-n",
    n,
    brief,
    candidates,
    review,
    apply_policy:
      "Parent applies winner only after reviewer PASS + parent re-verify.",
  };
}
