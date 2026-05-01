import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type InstallStatus = {
  status: string;
  targetPath: string;
  expectedPath: string;
  actualPath: string;
  message: string;
};

type ToolStatus = {
  id: string;
  label: string;
  status: string;
  paths: Record<string, string>;
  resources: Record<string, InstallStatus>;
};

type SkillItem = {
  name: string;
  description: string;
  path: string;
  file: string;
  installs: Record<string, string>;
};

type CommandItem = {
  name: string;
  description: string;
  path: string;
  installs: Record<string, string>;
};

type McpServer = {
  name: string;
  description: string;
  serverType: string;
  transport: string;
  url: string;
  command: string;
  args: string[];
  enabled: boolean;
  targets: Record<string, boolean>;
};

type McpInstallStatus = {
  tool: string;
  server: string;
  status: string;
  path: string;
  message: string;
};

type AppState = {
  repo?: {
    root: string;
    home: string;
    agentsHome: string;
    codexHome: string;
    paths: Record<string, string>;
  };
  repoError?: string;
  generatedAt: string;
  registry: {
    valid: boolean;
    path: string;
    error?: string;
    servers: McpServer[];
  };
  tools: ToolStatus[];
  skills: SkillItem[];
  commands: CommandItem[];
  mcpStatuses: Record<string, McpInstallStatus[]>;
};

type ScriptPlan = {
  action: string;
  title: string;
  cwd: string;
  command: string;
  args: string[];
  affectedPaths: string[];
  backupsMayBeCreated: boolean;
  note: string;
  displayCommand: string;
};

type ScriptResult = {
  ok: boolean;
  exitCode?: number;
  stdout: string;
  stderr: string;
  backups: string[];
};

type RepoImportPlan = {
  source: string;
  destination: string;
  sourceType: string;
  displayCommand: string;
  affectedPaths: string[];
  note: string;
};

type RepoImportResult = {
  ok: boolean;
  destination: string;
  repoPath?: string;
  stdout: string;
  stderr: string;
};

type DiffPreview = {
  action: string;
  title: string;
  sections: DiffSection[];
};

type DiffSection = {
  title: string;
  path: string;
  sectionType: string;
  status: string;
  diff: string;
};

const views = ["Dashboard", "Skills", "Commands", "MCP Servers", "Tools"] as const;
type View = (typeof views)[number];

const actions = [
  ["dryRunMcps", "Dry Run MCPs"],
  ["syncMcps", "Sync MCPs"],
  ["linkAgents", "Run Link Script"],
  ["dryRunAll", "Dry Run All"],
  ["syncAll", "Sync All"],
] as const;

const repoPathStorageKey = "agents-manager.repoPath";

function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [view, setView] = useState<View>("Dashboard");
  const [plan, setPlan] = useState<ScriptPlan | null>(null);
  const [preview, setPreview] = useState<DiffPreview | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [output, setOutput] = useState("No command has run yet.");
  const [meta, setMeta] = useState("Ready.");
  const [loading, setLoading] = useState(false);
  const [repoPath, setRepoPath] = useState(() => localStorage.getItem(repoPathStorageKey) ?? "");

  async function refresh(pathOverride = repoPath) {
    setLoading(true);
    try {
      setState(await invoke<AppState>("get_state", { repoPath: pathOverride || null }));
    } catch (error) {
      setOutput(String(error));
      setMeta("Failed to load state.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function reviewAction(action: string) {
    try {
      const [nextPlan, nextPreview] = await Promise.all([
        invoke<ScriptPlan>("plan_action", { action, repoPath: repoPath || null }),
        invoke<DiffPreview>("preview_action", { action, repoPath: repoPath || null }),
      ]);
      setPlan(nextPlan);
      setPreview(nextPreview);
    } catch (error) {
      setPlan(null);
      setPreview(null);
      setOutput(String(error));
      setMeta("Unable to plan command.");
    }
  }

  async function runAction(action: string) {
    setPlan(null);
    setPreview(null);
    setMeta(`Running ${action}...`);
    setOutput("Running...");
    try {
      const result = await invoke<ScriptResult>("run_action", { action, repoPath: repoPath || null });
      setMeta(`${action} exited with ${result.exitCode ?? "unknown"}`);
      setOutput(formatResult(result));
      await refresh();
    } catch (error) {
      setMeta("Command failed before start.");
      setOutput(String(error));
    }
  }

  async function chooseRepo() {
    try {
      const selected = await invoke<string | null>("choose_repo_path");
      if (!selected) return;
      localStorage.setItem(repoPathStorageKey, selected);
      setRepoPath(selected);
      await refresh(selected);
      setMeta("Repo override updated.");
    } catch (error) {
      setMeta("Repo selection failed.");
      setOutput(String(error));
    }
  }

  async function resetRepo() {
    localStorage.removeItem(repoPathStorageKey);
    setRepoPath("");
    await refresh("");
    setMeta("Using automatic repo detection.");
  }

  async function runRepoImport(source: string, destination: string) {
    setImportOpen(false);
    setMeta("Importing repo...");
    setOutput("Running import...");
    try {
      const result = await invoke<RepoImportResult>("run_repo_import", { source, destination });
      setMeta(result.ok ? "Repo import completed." : "Repo import finished with errors.");
      setOutput(formatImportResult(result));
      if (result.ok && result.repoPath) {
        localStorage.setItem(repoPathStorageKey, result.repoPath);
        setRepoPath(result.repoPath);
        await refresh(result.repoPath);
      }
    } catch (error) {
      setMeta("Repo import failed.");
      setOutput(String(error));
    }
  }

  async function openPath(path: string) {
    if (!path) return;
    try {
      await invoke("open_path", { path });
    } catch (error) {
      setMeta("Open path failed.");
      setOutput(String(error));
    }
  }

  const toolById = useMemo(() => Object.fromEntries((state?.tools ?? []).map((tool) => [tool.id, tool])), [state]);

  return (
    <div className="appShell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brandMark">A</div>
          <div>
            <h1>Agents Manager</h1>
            <p>Portable repo control</p>
          </div>
        </div>
        <nav className="nav">
          {views.map((name) => (
            <button key={name} className={view === name ? "active" : ""} onClick={() => setView(name)}>
              {name}
            </button>
          ))}
        </nav>
      </aside>

      <main className="content">
        <header className="topbar">
          <div>
            <div className="eyebrow">Source of truth {repoPath ? "(selected)" : "(auto-detected)"}</div>
            <div className="repoPath">{state?.repo?.root ?? state?.repoError ?? "Loading..."}</div>
          </div>
          <div className="buttonRow">
            <button className="ghostButton" onClick={chooseRepo}>
              Choose Repo
            </button>
            <button className="ghostButton" onClick={() => setImportOpen(true)}>
              Clone / Import
            </button>
            {repoPath && (
              <button className="ghostButton" onClick={resetRepo}>
                Reset Auto
              </button>
            )}
            <button className="ghostButton" onClick={() => refresh()} disabled={loading}>
              Refresh
            </button>
            <button className="ghostButton" onClick={() => state?.repo && openPath(state.repo.root)}>
              Open Repo
            </button>
          </div>
        </header>

        {!state?.repo ? (
          <Panel title="Repo Not Found" subtitle={state?.repoError ?? "Loading repository state."} />
        ) : (
          <>
            {view === "Dashboard" && <Dashboard state={state} toolById={toolById} onAction={reviewAction} />}
            {view === "Skills" && <Skills state={state} openPath={openPath} />}
            {view === "Commands" && <Commands state={state} openPath={openPath} />}
            {view === "MCP Servers" && <Mcps state={state} onAction={reviewAction} openPath={openPath} />}
            {view === "Tools" && <Tools state={state} onAction={reviewAction} openPath={openPath} />}
          </>
        )}

        <section className="terminalPanel">
          <div className="panelHeader">
            <div>
              <h2>Command Output</h2>
              <p>{meta}</p>
            </div>
            <button className="ghostButton" onClick={() => setOutput("")}>
              Clear
            </button>
          </div>
          <pre>{output}</pre>
        </section>
      </main>

      {plan && (
        <PlanDialog
          plan={plan}
          preview={preview}
          onCancel={() => {
            setPlan(null);
            setPreview(null);
          }}
          onRun={() => runAction(plan.action)}
        />
      )}
      {importOpen && (
        <ImportDialog
          defaultDestination={state?.repo?.home ? `${state.repo.home}/agents-import` : "~/.agents-import"}
          onCancel={() => setImportOpen(false)}
          onRun={runRepoImport}
        />
      )}
    </div>
  );
}

function Dashboard({ state, toolById, onAction }: { state: AppState; toolById: Record<string, ToolStatus>; onAction: (action: string) => void }) {
  return (
    <>
      <div className="summaryGrid">
        <Stat label="Claude Code" value={toolById["claude-code"]?.status ?? "unknown"} />
        <Stat label="Codex" value={toolById.codex?.status ?? "unknown"} />
        <Stat label="OpenCode" value={toolById.opencode?.status ?? "unknown"} />
        <Stat label="MCP registry" value={state.registry.valid ? "valid" : "invalid"} />
      </div>
      <Panel title="Sync Actions" subtitle="Each action shows command, working directory, affected paths, and backup behavior before it runs.">
        <div className="buttonRow padded">
          {actions.map(([action, label]) => (
            <button key={action} className="primaryButton" onClick={() => onAction(action)}>
              {label}
            </button>
          ))}
        </div>
      </Panel>
      <Panel title="Health" subtitle={`${state.skills.length} skills, ${state.commands.length} commands, ${state.registry.servers.length} MCP servers.`}>
        <table>
          <thead>
            <tr>
              <th>Tool</th>
              <th>Status</th>
              <th>Global Instructions</th>
              <th>Skills</th>
              <th>Commands</th>
            </tr>
          </thead>
          <tbody>
            {state.tools.map((tool) => (
              <tr key={tool.id}>
                <td>{tool.label}</td>
                <td><StatusPill value={tool.status} /></td>
                <td><ResourceCell resource={tool.resources.globalInstructions ?? tool.resources.envSnippet} /></td>
                <td><ResourceCell resource={tool.resources.skills} /></td>
                <td><ResourceCell resource={tool.resources.commands} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
    </>
  );
}

function Skills({ state, openPath }: { state: AppState; openPath: (path: string) => void }) {
  return (
    <Panel title="Skills" subtitle={`Read from ${state.repo?.paths.skills}`}>
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Description</th>
            <th>Claude Code</th>
            <th>Codex</th>
            <th>OpenCode</th>
            <th>Path</th>
          </tr>
        </thead>
        <tbody>
          {state.skills.map((skill) => (
            <tr key={skill.path}>
              <td>{skill.name}</td>
              <td className="description">{skill.description || "No frontmatter description."}</td>
              <td><StatusPill value={skill.installs["claude-code"]} /></td>
              <td><StatusPill value={skill.installs.codex} /></td>
              <td><StatusPill value={skill.installs.opencode} /></td>
              <td>
                <button className="smallButton" onClick={() => openPath(skill.file)}>Open</button>
                <div className="path">{skill.path}</div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Panel>
  );
}

function Commands({ state, openPath }: { state: AppState; openPath: (path: string) => void }) {
  return (
    <Panel title="Commands" subtitle={`Read from ${state.repo?.paths.commands}`}>
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Description</th>
            <th>Claude Code</th>
            <th>Codex Prompts</th>
            <th>OpenCode</th>
            <th>Path</th>
          </tr>
        </thead>
        <tbody>
          {state.commands.map((command) => (
            <tr key={command.path}>
              <td>/{command.name}</td>
              <td className="description">{command.description || "No frontmatter description."}</td>
              <td><StatusPill value={command.installs["claude-code"]} /></td>
              <td><StatusPill value={command.installs.codex} /></td>
              <td><StatusPill value={command.installs.opencode} /></td>
              <td>
                <button className="smallButton" onClick={() => openPath(command.path)}>Open</button>
                <div className="path">{command.path}</div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Panel>
  );
}

function Mcps({ state, onAction, openPath }: { state: AppState; onAction: (action: string) => void; openPath: (path: string) => void }) {
  return (
    <>
      <Panel title="MCP Servers" subtitle={`Registry: ${state.registry.path}`}>
        <div className="buttonRow padded compact">
          <button className="ghostButton" onClick={() => openPath(state.registry.path)}>Open Registry</button>
          <button className="primaryButton" onClick={() => onAction("dryRunMcps")}>Dry Run MCPs</button>
          <button className="primaryButton" onClick={() => onAction("syncMcps")}>Sync MCPs</button>
        </div>
        {!state.registry.valid ? (
          <div className="empty">{state.registry.error}</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Server</th>
                <th>Type</th>
                <th>Endpoint</th>
                <th>Enabled</th>
                <th>Targets</th>
                <th>Claude Code</th>
                <th>Codex</th>
                <th>OpenCode</th>
                <th>Auth</th>
              </tr>
            </thead>
            <tbody>
              {state.registry.servers.map((server) => (
                <tr key={server.name}>
                  <td>
                    {server.name}
                    <div className="description">{server.description}</div>
                  </td>
                  <td>{server.serverType}</td>
                  <td className="path">{server.url || [server.command, ...server.args].join(" ")}</td>
                  <td><StatusPill value={server.enabled ? "enabled" : "disabled"} /></td>
                  <td>{Object.entries(server.targets).filter(([, enabled]) => enabled !== false).map(([target]) => target).join(", ")}</td>
                  <td><McpCell status={findMcp(state, "claude-code", server.name)} /></td>
                  <td><McpCell status={findMcp(state, "codex", server.name)} /></td>
                  <td><McpCell status={findMcp(state, "opencode", server.name)} /></td>
                  <td><StatusPill value="unknown" /></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>
      <Panel title="Manual Auth Hints" subtitle="Run after MCP sync when the target tool requires login.">
        <div className="hintGrid">
          <code>codex mcp login atlassian</code>
          <code>opencode mcp auth atlassian</code>
          <span>Claude Code: open Claude Code and run <code>/mcp</code>.</span>
        </div>
      </Panel>
    </>
  );
}

function Tools({ state, onAction, openPath }: { state: AppState; onAction: (action: string) => void; openPath: (path: string) => void }) {
  return (
    <>
      <Panel title="Detected Tool Paths" subtitle="The GUI inspects these paths read-only. Apply actions are delegated to repo scripts.">
        <table>
          <thead>
            <tr>
              <th>Tool</th>
              <th>Status</th>
              <th>Path Kind</th>
              <th>Path</th>
              <th>Resource Status</th>
            </tr>
          </thead>
          <tbody>
            {state.tools.flatMap((tool) =>
              Object.entries(tool.paths).map(([key, value]) => (
                <tr key={`${tool.id}-${key}`}>
                  <td>{tool.label}</td>
                  <td><StatusPill value={tool.status} /></td>
                  <td>{labelize(key)}</td>
                  <td>
                    <button className="smallButton" onClick={() => openPath(value)}>Open</button>
                    <div className="path">{value}</div>
                  </td>
                  <td><StatusPill value={tool.resources[key]?.status ?? "n/a"} /></td>
                </tr>
              )),
            )}
          </tbody>
        </table>
      </Panel>
      <Panel title="Tool-Specific MCP Sync" subtitle="Runs the narrower MCP sync command for a single target tool.">
        <div className="buttonRow padded">
          <button className="primaryButton" onClick={() => onAction("syncClaudeCode")}>Sync Tool: Claude Code</button>
          <button className="primaryButton" onClick={() => onAction("syncCodex")}>Sync Tool: Codex</button>
          <button className="primaryButton" onClick={() => onAction("syncOpenCode")}>Sync Tool: OpenCode</button>
        </div>
      </Panel>
    </>
  );
}

function PlanDialog({
  plan,
  preview,
  onCancel,
  onRun,
}: {
  plan: ScriptPlan;
  preview: DiffPreview | null;
  onCancel: () => void;
  onRun: () => void;
}) {
  return (
    <div className="modalBackdrop">
      <div className="modal">
        <div className="panelHeader">
          <div>
            <div className="eyebrow">Review before running</div>
            <h2>{plan.title}</h2>
          </div>
        </div>
        <div className="modalBody">
          <Field label="Command to run"><pre className="commandBox">{plan.displayCommand}</pre></Field>
          <Field label="Working directory"><div className="path">{plan.cwd}</div></Field>
          <Field label="Files likely affected">
            <ul>{plan.affectedPaths.map((path) => <li key={path} className="path">{path}</li>)}</ul>
          </Field>
          <Field label="Backups">
            {plan.backupsMayBeCreated ? "Backups may be created by the underlying repo script." : "No backups should be created for this dry-run action."}
          </Field>
          <Field label="Note">{plan.note}</Field>
          <Field label="Preview">
            {preview ? (
              <div className="previewStack">
                {preview.sections.map((section) => (
                  <details className="previewSection" key={`${section.title}-${section.path}`} open={section.status === "changed" || section.status === "error"}>
                    <summary>
                      <span>{section.title}</span>
                      <StatusPill value={section.status} />
                    </summary>
                    <div className="path">{section.path}</div>
                    <pre className="diffBox">{section.diff}</pre>
                  </details>
                ))}
              </div>
            ) : (
              <div className="path">Preview unavailable.</div>
            )}
          </Field>
        </div>
        <div className="modalActions">
          <button className="ghostButton" onClick={onCancel}>Cancel</button>
          <button className="primaryButton" onClick={onRun}>Run Command</button>
        </div>
      </div>
    </div>
  );
}

function ImportDialog({
  defaultDestination,
  onCancel,
  onRun,
}: {
  defaultDestination: string;
  onCancel: () => void;
  onRun: (source: string, destination: string) => void;
}) {
  const [source, setSource] = useState("");
  const [destination, setDestination] = useState(defaultDestination);
  const [plan, setPlan] = useState<RepoImportPlan | null>(null);
  const [error, setError] = useState("");

  async function review() {
    setError("");
    try {
      setPlan(await invoke<RepoImportPlan>("plan_repo_import", { source, destination }));
    } catch (error) {
      setPlan(null);
      setError(String(error));
    }
  }

  return (
    <div className="modalBackdrop">
      <div className="modal">
        <div className="panelHeader">
          <div>
            <div className="eyebrow">Clone or import</div>
            <h2>Import Agents Repo</h2>
            <p>Supports Git URLs, ZIP URLs, and local ZIP files.</p>
          </div>
        </div>
        <div className="modalBody">
          <Field label="Source">
            <input
              className="textInput"
              value={source}
              onChange={(event) => {
                setSource(event.target.value);
                setPlan(null);
              }}
              placeholder="https://github.com/user/agents.git or https://.../repo.zip"
            />
          </Field>
          <Field label="Destination">
            <input
              className="textInput"
              value={destination}
              onChange={(event) => {
                setDestination(event.target.value);
                setPlan(null);
              }}
              placeholder="~/agents-import"
            />
          </Field>
          {error && <div className="errorBox">{error}</div>}
          {plan && (
            <>
              <Field label="Detected source type">
                <StatusPill value={plan.sourceType} />
              </Field>
              <Field label="Command / steps">
                <pre className="commandBox">{plan.displayCommand}</pre>
              </Field>
              <Field label="Files likely affected">
                <ul>{plan.affectedPaths.map((path) => <li key={path} className="path">{path}</li>)}</ul>
              </Field>
              <Field label="Note">{plan.note}</Field>
            </>
          )}
        </div>
        <div className="modalActions">
          <button className="ghostButton" onClick={onCancel}>Cancel</button>
          <button className="ghostButton" onClick={review}>Review</button>
          <button className="primaryButton" disabled={!plan} onClick={() => onRun(source, destination)}>Run Import</button>
        </div>
      </div>
    </div>
  );
}

function Panel({ title, subtitle, children }: { title: string; subtitle?: string; children?: React.ReactNode }) {
  return (
    <section className="panel">
      <div className="panelHeader">
        <div>
          <h2>{title}</h2>
          {subtitle && <p>{subtitle}</p>}
        </div>
      </div>
      {children}
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="field">
      <div className="eyebrow">{label}</div>
      {children}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="statCard">
      <div className="eyebrow">{label}</div>
      <StatusPill value={value} />
    </div>
  );
}

function StatusPill({ value }: { value?: string }) {
  const normalized = value || "unknown";
  return <span className={`status ${normalized}`}>{normalized}</span>;
}

function ResourceCell({ resource }: { resource?: InstallStatus }) {
  if (!resource) return <span className="path">n/a</span>;
  return (
    <>
      <StatusPill value={resource.status} />
      <div className="path">{resource.targetPath}</div>
    </>
  );
}

function McpCell({ status }: { status?: McpInstallStatus }) {
  if (!status) return <StatusPill value="unknown" />;
  return (
    <>
      <StatusPill value={status.status} />
      <div className="path">{status.message}</div>
    </>
  );
}

function findMcp(state: AppState, tool: string, server: string) {
  return state.mcpStatuses[tool]?.find((status) => status.server === server);
}

function labelize(value: string) {
  return value.replace(/([A-Z])/g, " $1").replace(/^./, (char) => char.toUpperCase());
}

function formatResult(result: ScriptResult) {
  return [
    `ok: ${result.ok}`,
    `exit: ${result.exitCode ?? "unknown"}`,
    "",
    "stdout:",
    result.stdout.trimEnd() || "<empty>",
    "",
    "stderr:",
    result.stderr.trimEnd() || "<empty>",
    result.backups.length ? `\nbackups:\n${result.backups.join("\n")}` : "",
  ].join("\n");
}

function formatImportResult(result: RepoImportResult) {
  return [
    `ok: ${result.ok}`,
    `destination: ${result.destination}`,
    result.repoPath ? `repo: ${result.repoPath}` : "repo: <not selected>",
    "",
    "stdout:",
    result.stdout.trimEnd() || "<empty>",
    "",
    "stderr:",
    result.stderr.trimEnd() || "<empty>",
  ].join("\n");
}

createRoot(document.getElementById("root")!).render(<App />);
