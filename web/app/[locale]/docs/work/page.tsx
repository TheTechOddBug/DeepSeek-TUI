import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  return buildPageMetadata({
    path: "/docs/work",
    locale,
    title: isZh ? "工作面板 · Codewhale 文档" : "Work Surface · Codewhale Docs",
    description: isZh
      ? "唯一的 To-do 执行台账、模型可见的 Work grounding，以及同一份工作状态的延续路径。"
      : "The single canonical To-do ledger, model-facing Work grounding, and how one work state stays continuous.",
  });
}

export default async function WorkSurfacePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  const bodyClass = isZh
    ? "text-ink-soft leading-[1.9] tracking-wide"
    : "text-ink-soft leading-relaxed";

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h2 className="font-display text-3xl mb-1">{isZh ? "工作面板" : "The Work surface"}</h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "Codewhale 的 TUI 侧栏有一块 Work 区域，显示当前工作的实时状态。它不只是视觉上的待办清单：同一份工作状态同时由模型可见的工具、会话接力（relay）和子 Agent 交接共同维护。Codewhale 只有一个 Work 面板——带计数的 To-do 执行台账。update_plan 是对话式的推理笔记，不是第二个进度面板。"
            : "The TUI sidebar has a Work area that shows live state for the current job. It is more than a visual to-do list: the same work state is maintained by model-visible tools, session relay, and sub-agent handoff. Codewhale has exactly one Work surface — the counted To-do execution ledger. update_plan is conversational reasoning, not a second progress surface."}
        </p>
      </section>

      <section id="checklist" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">
          {isZh ? "To-do：唯一的执行台账" : "To-do: the sole canonical ledger"}
        </h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh ? (
            <>
              To-do 是具体工作的进度台账：一组带状态的条目（pending / in_progress / completed /
              cancelled），外加完成百分比和当前进行中的条目。模型通过 canonical 的{" "}
              <code className="inline">todo_write</code> 工具替换活动线程或持久任务的
              To-do 投影——这是模型可见的进度表面。旧的{" "}
              <code className="inline">checklist_*</code> 和 <code className="inline">todo_*</code>{" "}
              名字仍是隐藏的兼容别名：它们对同一份 To-do 状态保持可派发，以便旧 transcript
              回放，但不会出现在模型目录里。
            </>
          ) : (
            <>
              The To-do is the progress ledger for concrete work: a list of items with status
              (pending / in_progress / completed / cancelled), a completion percentage, and the item
              currently in progress. The model replaces this projection for the active thread or
              durable task through the canonical <code className="inline">todo_write</code> tool —
              the model-visible progress surface. The legacy{" "}
              <code className="inline">checklist_*</code> and <code className="inline">todo_*</code>{" "}
              names remain hidden compatibility aliases: they stay dispatchable against the same To-do
              state so old transcripts replay, but they are not advertised to the model catalog.
            </>
          )}
        </p>
      </section>

      <section id="strategy" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">
          {isZh ? "策略是对话式推理：update_plan" : "Strategy is conversational reasoning: update_plan"}
        </h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "update_plan 承载的是可选的高层策略，不是第二个台账。它的字段面向阶段级理解：标题、目标、上下文摘要、说明、来源、关键文件、约束、推荐方案、验证计划、风险与未知、交接包，以及一组步骤。它帮助父会话或后续 worker 理解“为什么这么做”；具体执行进度始终属于 To-do 台账。侧栏有意不把策略状态渲染成第二条进度列表，模型可见的 Work grounding 也不会包含它——只有 update_plan 而 To-do 为空时，不会产生任何 Work 状态。"
            : "update_plan carries optional high-level strategy — it is not a second ledger. Its fields serve phase-level understanding: title, objective, context summary, explanation, sources, critical files, constraints, recommended approach, verification plan, risks and unknowns, a handoff packet, and a list of steps. It helps a parent session or a later worker understand the approach; concrete execution progress always belongs to the To-do ledger. The sidebar deliberately does not render strategy state as a second progress list, and model-facing Work grounding excludes it entirely — plan state with an empty To-do produces no Work state at all."}
        </p>
      </section>

      <section id="continuity" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">
          {isZh ? "延续性：同一份状态流向各处" : "Continuity: one state, many surfaces"}
        </h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "同一份工作状态喂给多个出口，而且用的是同一个渲染器：每个父回合循环和子 Agent 步骤请求的尾部会附加一个瞬时的 <codewhale:work_state> 块；分叉（fork_context）的子 Agent 在其前缀的结构化状态块里收到同样的正文；/relay 把同样的正文写进交接指令。三处的 To-do 正文逐字节一致——子 Agent 与下一个线程因此从父级真实的进度位置继续，而不是从转述的摘要开始。侧栏的 To-do 区域则实时渲染同一份状态。"
            : "The same work state feeds several surfaces through one renderer: a transient <codewhale:work_state> block is appended to each parent turn-loop and sub-agent step request; a forked (fork_context) sub-agent receives the same body inside its structured state block; and /relay writes the same body into the handoff instruction. The To-do body is byte-identical in all three, so a child agent and the next thread continue from the parent's real progress position instead of a paraphrased summary. The sidebar renders that same state live."}
        </p>
      </section>

      <section id="capture" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">
          {isZh ? "终端实拍（文本复原）" : "Terminal capture (faithful text)"}
        </h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "下面的文本块按 crates/tui/src/tui/sidebar.rs 的渲染逻辑逐行复原侧栏 Work 区域：目标是带 ◆ 图标的 Goal 行、耗时、token 预算条；然后是完成度计数和带编号的状态条目。"
            : "This text block reproduces the sidebar Work area line-for-line from the rendering logic in crates/tui/src/tui/sidebar.rs: the goal row with its ◆ icon, elapsed time, and token budget bar, then the settled counter and the numbered status items."}
        </p>
        <pre className="code-block mt-4">{`To-do
◆ Goal: Land the v0.9.2 website docs cluster
elapsed: 18m
[█████████░░░░░░░░░░░] 45%
50% settled (2/4)
[✓] #1 Read docs-map.ts and the Modes page pattern
[✓] #2 Draft the Fleet and Sandbox pages
[~] #3 Write the Work surface page
[ ] #4 Run check:docs, tests, and the build`}</pre>
        <p className={`${bodyClass} mt-3`}>
          {isZh ? (
            <>
              条目前缀对应四种状态：<code className="inline">[ ]</code> 待办、
              <code className="inline">[~]</code> 进行中、<code className="inline">[✓]</code> 完成、
              <code className="inline">[-]</code> 取消。空间不够时侧栏窗口化到进行中条目附近，并用
              “+N more To-do items” 标注被省略的条目。
            </>
          ) : (
            <>
              The item prefixes map to the four statuses: <code className="inline">[ ]</code> pending,{" "}
              <code className="inline">[~]</code> in progress, <code className="inline">[✓]</code>{" "}
              completed, <code className="inline">[-]</code> cancelled. When space runs out, the sidebar
              windows around the in-progress item and marks the omission with “+N more To-do items”.
            </>
          )}
        </p>
      </section>

      <section id="model-facing" className="scroll-mt-32">
        <h2 className="font-display text-2xl mb-1">
          {isZh ? "哪些是模型可见的，哪些只是界面" : "What is model-facing vs. visual-only"}
        </h2>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "已被实现和测试证实的模型可见路径有五条：todo_write 工具本身是模型目录里的活跃工具；每个父回合循环请求尾部的 <codewhale:work_state> 块（#3983）；每个子 Agent 步骤请求尾部的同一个块——渲染自它自己的清单；分叉子 Agent 的结构化状态块（<codewhale:fork_state> 中的 Work 小节，在真正 fork 的那一刻解析）；以及 /relay 输出。侧栏渲染是视觉呈现——它给人看，不注入模型上下文。"
            : "Five model-facing paths are implemented and covered by tests: the todo_write tool itself, which is active in the model catalog; the <codewhale:work_state> block appended to each parent turn-loop request (#3983); the same block on each sub-agent step request, rendered from that agent's own list; the forked sub-agent's structured state block (the Work section inside <codewhale:fork_state>, resolved at the moment of the fork); and /relay output. The sidebar rendering is a visual presentation — it informs the operator and is not injected into model context."}
        </p>
        <p className={`${bodyClass} mt-3`}>
          {isZh
            ? "边界值得说清楚：这个块是瞬时的——它只属于当次请求，既不写进会话历史，也不进入稳定系统前缀，因此稳定的系统与工具前缀仍可参与前缀缓存；各提供商对最新用户消息的缓存方式仍以其自身协议为准。它会在每个父回合循环和子 Agent 步骤请求前重建，读取的是权威状态（有 work graph 时读它暂存的投影，而不是尚未发布的旧视图），所以工具循环中途的一次 todo_write 会在下一步出现。父回合循环的上下文预检按真正会发出的那一份尾部计费，因此不会先放行、再因为附加这个块而超限；离线计数一律偏保守。条目数与字符数都有硬上限，进行中的条目优先保留，被省略的部分带省略标记。To-do 为空时不输出任何块。渲染器只保证包裹结构、控制字符与上限这三件事——它不会审查条目文本的含义，任意 To-do 内容不因此变成可信指令。"
            : "The boundaries are worth stating: the block is transient — it belongs to a single request, is never written to session history, and never enters the stable system prefix, so the stable system-and-tool prefix remains eligible for prefix caching; each provider's treatment of the latest user message still depends on its wire protocol. It is rebuilt before each parent turn-loop and sub-agent step request from the authoritative state (the work graph's staged projection where one exists, not the not-yet-published legacy view), so a todo_write made mid tool-loop appears on the following step. The parent turn-loop context preflight is charged for the exact tail that will be sent, so it cannot approve a request that goes over-limit only once the block is appended; offline counts stay conservative. Item count and character count are both hard-bounded, the in-progress item is preserved preferentially, and elided content is marked. An empty To-do emits no block at all. The renderer guarantees exactly three things — wrapper framing cannot be closed early, control characters cannot forge the line format, and the bounds hold. It does not vet what item text says, so arbitrary To-do content is not thereby made safe to follow as instructions."}
        </p>
      </section>

      <section id="source" className="hairline-t pt-8">
        <p className="text-sm text-ink-mute">
          {isZh
            ? "来源文档：docs/TOOL_SURFACE.md, docs/TOOL_LIFECYCLE.md · 更新时请同步修改 docs-map.ts。"
            : "Source documents: docs/TOOL_SURFACE.md, docs/TOOL_LIFECYCLE.md · Update docs-map.ts when changing."}
        </p>
      </section>
    </section>
  );
}
