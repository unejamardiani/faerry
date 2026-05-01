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

type BundledScriptInfo = {
  name: string;
  checksum: string;
  size: number;
};

type RuntimeInfo = {
  appVersion: string;
  repoRoot: string;
  repoMode: string;
  scriptFamily: string;
  platform: string;
  platformArch: string;
  dependencies: Record<string, string>;
  scripts: BundledScriptInfo[];
  repoHasLocalScripts: boolean;
  agentsMdExists: boolean;
  agentsMdSize: number;
  agentsMdModified: string;
};

type ScriptVersionInfo = {
  name: string;
  bundledChecksum: string;
  repoPath: string;
  repoChecksum: string | null;
  status: "bundledMatchesRepo" | "bundledDiffersFromRepo" | "repoMissing" | "noRepoScript";
};

type AgentsMdInfo = {
  path: string;
  exists: boolean;
  size: number;
  lastModified: string | null;
  content: string;
  valid: boolean;
  issues: string[];
};

type ValidationIssue = {
  code: string;
  path: string;
  message: string;
  suggestion: string;
  severity: string;
};

type UpdateCheckResult = {
  currentVersion: string;
  latestVersion?: string;
  updateUrl?: string;
  upToDate: boolean;
  note: string;
};

type RepoValidation = {
  path: string;
  issues: ValidationIssue[];
  severitySummary: { info: number; warning: number; error: number };
};

type LogEntry = {
  timestamp: string;
  action: string;
  repoPath: string;
  command: string;
  exitCode?: number;
  ok: boolean;
  stdout: string;
  stderr: string;
  backups: string[];
};

type Profile = {
  name: string;
  description: string;
  toolsEnabled: string[];
  syncGlobals: boolean;
  syncSkills: boolean;
  syncCommands: boolean;
  syncMcp: boolean;
  selectedMcpServers: string[];
  selectedSkills: string[];
  selectedCommands: string[];
};

type McpServerFormData = {
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

type McpRegistryEditResult = {
  ok: boolean;
  message: string;
  validationErrors: string[];
  diff: string;
};

type CreateResult = { ok: boolean; path: string; message: string };

type PackageResult = {
  ok: boolean;
  message: string;
  artifacts: { name: string; path: string; size: number }[];
  stdout: string;
  stderr: string;
};

type StructuredOutput = {
  summary: string[];
  changed: string[];
  skipped: string[];
  warnings: string[];
  errors: string[];
  backups: string[];
  authHints: string[];
  rawStdout: string;
  rawStderr: string;
  exitCode?: number;
};

const views = ["Dashboard", "Skills", "Commands", "MCP Servers", "Tools", "About", "Validation", "Editor", "Logs", "Profiles"] as const;
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
  const [runtimeInfo, setRuntimeInfo] = useState<RuntimeInfo | null>(null);
  const [scriptVersions, setScriptVersions] = useState<ScriptVersionInfo[]>([]);
  const [agentsMdInfo, setAgentsMdInfo] = useState<AgentsMdInfo | null>(null);
  const [safetyWarnings, setSafetyWarnings] = useState<string[]>([]);
  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [repoValidation, setRepoValidation] = useState<RepoValidation | null>(null);
  const [validating, setValidating] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedSkill, setSelectedSkill] = useState<SkillItem | null>(null);
  const [selectedCommand, setSelectedCommand] = useState<CommandItem | null>(null);
  const [selectedMcp, setSelectedMcp] = useState<McpServer | null>(null);
  const [structuredOutput, setStructuredOutput] = useState<StructuredOutput | null>(null);

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

  async function loadAboutInfo() {
    const rp = repoPath || null;
    try {
      const [rt, sv, am, sw] = await Promise.all([
        invoke<RuntimeInfo>("get_runtime_info", { repoPath: rp }),
        invoke<ScriptVersionInfo[]>("get_script_versions", { repoPath: rp }),
        invoke<AgentsMdInfo>("get_agents_md_info", { repoPath: rp }),
        invoke<string[]>("check_safety_guards", { repoPath: rp }),
      ]);
      setRuntimeInfo(rt);
      setScriptVersions(sv);
      setAgentsMdInfo(am);
      setSafetyWarnings(sw);
      setUpdateResult(null);
    } catch (error) {
      setOutput(String(error));
      setMeta("Failed to load About info.");
    }
  }

  async function openAgentsMd() {
    if (!agentsMdInfo?.path) return;
    await openPath(agentsMdInfo.path);
  }

  async function checkForUpdates() {
    setCheckingUpdate(true);
    try {
      setUpdateResult(await invoke<UpdateCheckResult>("check_for_updates"));
    } catch (error) {
      setOutput(String(error));
    } finally {
      setCheckingUpdate(false);
    }
  }

  async function validateRepo() {
    setValidating(true);
    setRepoValidation(null);
    try {
      setRepoValidation(await invoke<RepoValidation>("validate_repo", { repoPath: repoPath || null }));
    } catch (error) {
      setOutput(String(error));
      setMeta("Validation failed.");
    } finally {
      setValidating(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    if (view === "About") loadAboutInfo();
    if (view === "Logs") loadLogs();
    if (view === "Profiles") loadProfiles();
  }, [view]);

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
    setStructuredOutput(null);
    setMeta(`Running ${action}...`);
    setOutput("Running...");
    try {
      const result = await invoke<ScriptResult>("run_action", { action, repoPath: repoPath || null });
      try {
        const parsed = await invoke<StructuredOutput>("parse_script_output", {
          stdout: result.stdout,
          stderr: result.stderr,
          exitCode: result.exitCode ?? null,
          backups: result.backups,
        });
        setStructuredOutput(parsed);
      } catch {}
      try {
        await invoke("log_action", {
          action,
          repoPath: repoPath || "",
          command: action,
          ok: result.ok,
          exitCode: result.exitCode ?? null,
          stdout: result.stdout,
          stderr: result.stderr,
          backups: result.backups,
        });
      } catch {}
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

  async function loadLogs() {
    try {
      setLogs(await invoke<LogEntry[]>("get_logs"));
    } catch {}
  }

  async function clearLogs() {
    try {
      await invoke("clear_logs_cmd");
      setLogs([]);
    } catch {}
  }

  async function loadProfiles() {
    try {
      setProfiles(await invoke<Profile[]>("get_profiles", { repoPath: repoPath || null }));
    } catch {}
  }

  const toolById = useMemo(() => Object.fromEntries((state?.tools ?? []).map((tool: ToolStatus) => [tool.id, tool])), [state]);

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
            {view === "About" && <About runtimeInfo={runtimeInfo} scriptVersions={scriptVersions} agentsMdInfo={agentsMdInfo} safetyWarnings={safetyWarnings} updateResult={updateResult} checkingUpdate={checkingUpdate} onCheckUpdates={checkForUpdates} onOpenAgentsMd={openAgentsMd} />}
            {view === "Validation" && <Validation onValidate={validateRepo} repoValidation={repoValidation} validating={validating} />}
            {view === "Logs" && <Logs logs={logs} onClear={clearLogs} onRefresh={loadLogs} />}
            {view === "Profiles" && <Profiles profiles={profiles} />}
            {view === "Editor" && <Editor state={state} repoPath={repoPath} onRefresh={refresh} />}
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

function About({
  runtimeInfo,
  scriptVersions,
  agentsMdInfo,
  safetyWarnings,
  updateResult,
  checkingUpdate,
  onCheckUpdates,
  onOpenAgentsMd,
}: {
  runtimeInfo: RuntimeInfo | null;
  scriptVersions: ScriptVersionInfo[];
  agentsMdInfo: AgentsMdInfo | null;
  safetyWarnings: string[];
  updateResult: UpdateCheckResult | null;
  checkingUpdate: boolean;
  onCheckUpdates: () => void;
  onOpenAgentsMd: () => void;
}) {
  return (
    <>
      <Panel title="About" subtitle="Application runtime info, bundled scripts, and environment details.">
        {!runtimeInfo ? (
          <div className="empty">Loading runtime info...</div>
        ) : (
          <div className="aboutGrid">
            <div>
              <Field label="App Version">{runtimeInfo.appVersion}</Field>
              <Field label="Platform">{runtimeInfo.platform} ({runtimeInfo.platformArch})</Field>
              <Field label="Repo Mode">
                <StatusPill value={runtimeInfo.repoMode} />
              </Field>
              <Field label="Script Family">{runtimeInfo.scriptFamily}</Field>
            </div>
            <div>
              <Field label="Repo Root"><div className="path">{runtimeInfo.repoRoot}</div></Field>
              <Field label="AGENTS.md">
                {runtimeInfo.agentsMdExists
                  ? `${runtimeInfo.agentsMdSize} bytes, modified ${runtimeInfo.agentsMdModified}`
                  : "Not found"}
              </Field>
            </div>
          </div>
        )}
      </Panel>

      <Panel title="Dependencies" subtitle="Required toolchain for the bundled scripts to work.">
        {!runtimeInfo ? (
          <div className="empty">Loading...</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Dependency</th>
                <th>Version / Path</th>
              </tr>
            </thead>
            <tbody>
              {Object.entries(runtimeInfo.dependencies).map(([name, version]) => (
                <tr key={name}>
                  <td>{labelize(name)}</td>
                  <td className="path">{version}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      <Panel title="Bundled Scripts" subtitle="Scripts shipped with the application, with checksums for integrity verification.">
        {!runtimeInfo ? (
          <div className="empty">Loading...</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Size</th>
                <th>Checksum (SHA-256)</th>
              </tr>
            </thead>
            <tbody>
              {runtimeInfo.scripts.map((script) => (
                <tr key={script.name}>
                  <td>{script.name}</td>
                  <td>{script.size} bytes</td>
                  <td className="path">{script.checksum}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      {scriptVersions.length > 0 && (
        <Panel title="Script Versions" subtitle={runtimeInfo?.repoHasLocalScripts ? "Repo scripts present — compared against bundled copies." : "No repo scripts; showing bundled versions only."}>
          <table>
            <thead>
              <tr>
                <th>Script</th>
                <th>Bundled Checksum</th>
                <th>Repo Checksum</th>
                <th>Status</th>
                <th>Repo Path</th>
              </tr>
            </thead>
            <tbody>
              {scriptVersions.map((sv) => (
                <tr key={sv.name}>
                  <td>{sv.name}</td>
                  <td className="path">{sv.bundledChecksum}</td>
                  <td className="path">{sv.repoChecksum ?? "—"} </td>
                  <td><StatusPill value={sv.status} /></td>
                  <td className="path">{sv.repoPath}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Panel>
      )}

      {safetyWarnings.length > 0 && (
        <Panel title="Safety Warnings" subtitle="Issues detected by the safety guard checks.">
          <ul className="warningList">
            {safetyWarnings.map((w) => (
              <li key={w} className="warningItem">{w}</li>
            ))}
          </ul>
        </Panel>
      )}

      {agentsMdInfo && agentsMdInfo.exists && (
        <div className="buttonRow padded">
          <button className="primaryButton" onClick={onOpenAgentsMd}>Open AGENTS.md</button>
        </div>
      )}

      <Panel title="Updates" subtitle="Check for newer versions of the Agents Manager application.">
        <div className="buttonRow padded compact">
          <button className="primaryButton" onClick={onCheckUpdates} disabled={checkingUpdate}>
            {checkingUpdate ? "Checking..." : "Check for Updates"}
          </button>
        </div>
        {updateResult && (
          <div className="aboutGrid">
            <div>
              <Field label="Current Version">{updateResult.currentVersion}</Field>
              <Field label="Latest Version">{updateResult.latestVersion ?? "unknown"}</Field>
              <Field label="Update Available">
                <StatusPill value={updateResult.upToDate ? "up-to-date" : "available"} />
              </Field>
            </div>
            {updateResult.updateUrl && (
              <div>
                <Field label="Download URL"><div className="path">{updateResult.updateUrl}</div></Field>
                <Field label="Note">{updateResult.note}</Field>
              </div>
            )}
          </div>
        )}
      </Panel>
    </>
  );
}

function Validation({
  onValidate,
  repoValidation,
  validating,
}: {
  onValidate: () => void;
  repoValidation: RepoValidation | null;
  validating: boolean;
}) {
  const issues = repoValidation?.issues ?? [];
  const bySeverity = useMemo<Record<string, ValidationIssue[]>>(() => {
    const grouped: Record<string, ValidationIssue[]> = { error: [], warning: [], info: [] };
    for (const issue of issues) {
      const key = issue.severity.toLowerCase() || "info";
      if (grouped[key]) grouped[key].push(issue);
      else (grouped.info ??= []).push(issue);
    }
    return grouped;
  }, [issues]);

  return (
    <>
      <Panel title="AGENTS.md & Repo Validation" subtitle="Runs deep checks on the AGENTS.md file and repo configuration.">
        <div className="buttonRow padded compact">
          <button className="primaryButton" onClick={onValidate} disabled={validating}>
            {validating ? "Validating..." : "Run Validation"}
          </button>
        </div>
        {repoValidation && (
          <div className="aboutGrid">
            <Field label="Checked Path"><div className="path">{repoValidation.path}</div></Field>
            <Field label="Summary">
              <span className="path">
                {repoValidation.severitySummary.error} errors, {repoValidation.severitySummary.warning} warnings, {repoValidation.severitySummary.info} info
              </span>
            </Field>
          </div>
        )}
      </Panel>

      {issues.length === 0 && !validating ? (
        <Panel title="Results" subtitle="No issues found or validation not yet run.">
          <div className="empty">Click "Run Validation" to check the repository.</div>
        </Panel>
      ) : (
        Object.entries(bySeverity).map(([severity, sevIssues]) =>
          sevIssues.length > 0 ? (
            <Panel
              key={severity}
              title={`${labelize(severity)}s (${sevIssues.length})`}
              subtitle={severity === "error" ? "These issues should be fixed." : severity === "warning" ? "Review these warnings." : "Informational items."}
            >
              <table>
                <thead>
                  <tr>
                    <th>Code</th>
                    <th>Path</th>
                    <th>Message</th>
                    <th>Suggestion</th>
                  </tr>
                </thead>
                <tbody>
                  {sevIssues.map((issue, i) => (
                    <tr key={`${issue.code}-${i}`}>
                      <td><code>{issue.code}</code></td>
                      <td className="path">{issue.path}</td>
                      <td>{issue.message}</td>
                      <td>{issue.suggestion}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Panel>
          ) : null
        )
      )}
    </>
  );
}

function Logs({ logs, onClear, onRefresh }: { logs: LogEntry[]; onClear: () => void; onRefresh: () => void }) {
  return (
    <Panel title="Action Logs" subtitle={`${logs.length} logged actions.`}>
      <div className="buttonRow padded compact">
        <button className="ghostButton" onClick={onRefresh}>Refresh</button>
        <button className="ghostButton" onClick={onClear}>Clear Logs</button>
      </div>
      {logs.length === 0 ? (
        <div className="empty">No logs yet. Logs are recorded when you run sync actions.</div>
      ) : (
        <div className="logList">
          {[...logs].reverse().map((entry, i) => (
            <details key={i} className="logEntry">
              <summary>
                <StatusPill value={entry.ok ? "ok" : "error"} />
                <span className="logAction">{entry.action}</span>
                <span className="logTime">{new Date(Number(entry.timestamp) * 1000).toLocaleString()}</span>
                {entry.exitCode !== undefined && <span className="path">exit: {entry.exitCode}</span>}
              </summary>
              <div className="logBody">
                <Field label="Repo Path"><div className="path">{entry.repoPath || "—"}</div></Field>
                <Field label="Command"><code>{entry.command}</code></Field>
                {entry.backups.length > 0 && (
                  <Field label="Backups">
                    <ul>{entry.backups.map((b) => <li key={b} className="path">{b}</li>)}</ul>
                  </Field>
                )}
                {entry.stdout && (
                  <Field label="stdout"><pre className="logOutput">{entry.stdout}</pre></Field>
                )}
                {entry.stderr && (
                  <Field label="stderr"><pre className="logOutput">{entry.stderr}</pre></Field>
                )}
              </div>
            </details>
          ))}
        </div>
      )}
    </Panel>
  );
}

function Profiles({ profiles }: { profiles: Profile[] }) {
  return (
    <>
      <Panel title="Profiles" subtitle="Sync profiles define which tools and resources are managed.">
        {profiles.length === 0 ? (
          <div className="empty">No profiles loaded. Open a repo to view profiles.</div>
        ) : null}
      </Panel>
      {profiles.map((profile) => (
        <Panel key={profile.name} title={profile.name} subtitle={profile.description}>
          <div className="aboutGrid">
            <div>
              <Field label="Tools Enabled">{profile.toolsEnabled.join(", ") || "None"}</Field>
              <Field label="Sync Skills"><StatusPill value={profile.syncSkills ? "yes" : "no"} /></Field>
              <Field label="Sync Commands"><StatusPill value={profile.syncCommands ? "yes" : "no"} /></Field>
            </div>
            <div>
              <Field label="Sync Globals"><StatusPill value={profile.syncGlobals ? "yes" : "no"} /></Field>
              <Field label="Sync MCP"><StatusPill value={profile.syncMcp ? "yes" : "no"} /></Field>
              {profile.selectedSkills.length > 0 && (
                <Field label="Selected Skills">{profile.selectedSkills.join(", ")}</Field>
              )}
              {profile.selectedMcpServers.length > 0 && (
                <Field label="Selected MCP Servers">{profile.selectedMcpServers.join(", ")}</Field>
              )}
            </div>
          </div>
        </Panel>
      ))}
    </>
  );
}

type EditorTab = "MCP Editor" | "Skill Wizard" | "Command Wizard" | "Packaging";
const editorTabs: EditorTab[] = ["MCP Editor", "Skill Wizard", "Command Wizard", "Packaging"];

function Editor({ state, repoPath, onRefresh }: { state: AppState; repoPath: string; onRefresh: () => void }) {
  const [tab, setTab] = useState<EditorTab>("MCP Editor");
  return (
    <>
      <Panel title="Editor" subtitle="Create and modify repo resources.">
        <div className="buttonRow padded compact">
          {editorTabs.map((t) => (
            <button key={t} className={tab === t ? "primaryButton" : "ghostButton"} onClick={() => setTab(t)}>
              {t}
            </button>
          ))}
        </div>
      </Panel>
      {tab === "MCP Editor" && <McpEditor state={state} repoPath={repoPath} onRefresh={onRefresh} />}
      {tab === "Skill Wizard" && <SkillWizard repoPath={repoPath} onRefresh={onRefresh} />}
      {tab === "Command Wizard" && <CommandWizard repoPath={repoPath} onRefresh={onRefresh} />}
      {tab === "Packaging" && <Packaging repoPath={repoPath} />}
    </>
  );
}

function McpEditor({ state, repoPath, onRefresh }: { state: AppState; repoPath: string; onRefresh: () => void }) {
  const [action, setAction] = useState<"add" | "edit" | "delete">("add");
  const [selectedServer, setSelectedServer] = useState("");
  const [form, setForm] = useState<McpServerFormData>({
    name: "", description: "", serverType: "remote", transport: "http",
    url: "", command: "", args: [], enabled: true, targets: {},
  });
  const [result, setResult] = useState<McpRegistryEditResult | null>(null);
  const [running, setRunning] = useState(false);

  function selectServer(name: string) {
    setSelectedServer(name);
    const server = state.registry.servers.find((s) => s.name === name);
    if (server) {
      setForm({
        name: server.name,
        description: server.description || "",
        serverType: server.serverType || "remote",
        transport: server.transport || "http",
        url: server.url || "",
        command: server.command || "",
        args: server.args || [],
        enabled: server.enabled,
        targets: server.targets || {},
      });
    }
  }

  async function submit() {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<McpRegistryEditResult>("edit_mcp_server", {
        registryPath: state.registry.path,
        name: action === "add" ? form.name : selectedServer,
        action,
        data: action === "delete" ? null : form,
      });
      setResult(res);
      if (res.ok) onRefresh();
    } catch (error) {
      setResult({ ok: false, message: String(error), validationErrors: [], diff: "" });
    } finally {
      setRunning(false);
    }
  }

  return (
    <Panel title="MCP Registry Editor" subtitle={`Editing: ${state.registry.path}`}>
      <div className="buttonRow padded compact">
        {(["add", "edit", "delete"] as const).map((a) => (
          <button key={a} className={action === a ? "primaryButton" : "ghostButton"} onClick={() => setAction(a)}>
            {labelize(a)}
          </button>
        ))}
      </div>

      {(action === "edit" || action === "delete") && (
        <Field label="Select Server">
          <select className="textInput" value={selectedServer} onChange={(e) => selectServer(e.target.value)}>
            <option value="">— choose —</option>
            {state.registry.servers.map((s) => <option key={s.name} value={s.name}>{s.name}</option>)}
          </select>
        </Field>
      )}

      {action !== "delete" && (
        <>
          <Field label="Name">
            <input className="textInput" value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="my-server" disabled={action === "edit"} />
          </Field>
          <Field label="Description">
            <input className="textInput" value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} placeholder="Optional description" />
          </Field>
          <Field label="Type">
            <select className="textInput" value={form.serverType} onChange={(e) => setForm({ ...form, serverType: e.target.value })}>
              <option value="remote">remote</option>
              <option value="stdio">stdio</option>
            </select>
          </Field>
          <Field label="Transport">
            <select className="textInput" value={form.transport} onChange={(e) => setForm({ ...form, transport: e.target.value })}>
              <option value="http">http</option>
              <option value="sse">sse</option>
              <option value="stdio">stdio</option>
            </select>
          </Field>
          {form.serverType === "remote" ? (
            <Field label="URL">
              <input className="textInput" value={form.url} onChange={(e) => setForm({ ...form, url: e.target.value })} placeholder="https://..." />
            </Field>
          ) : (
            <>
              <Field label="Command">
                <input className="textInput" value={form.command} onChange={(e) => setForm({ ...form, command: e.target.value })} placeholder="npx" />
              </Field>
              <Field label="Args (space-separated)">
                <input className="textInput" value={form.args.join(" ")} onChange={(e) => setForm({ ...form, args: e.target.value.split(" ").filter(Boolean) })} placeholder="-y some-package" />
              </Field>
            </>
          )}
          <Field label="Enabled">
            <input type="checkbox" checked={form.enabled} onChange={(e) => setForm({ ...form, enabled: e.target.checked })} />
          </Field>
        </>
      )}

      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          <div>{result.message}</div>
          {result.validationErrors.map((e, i) => <div key={i} className="path">{e}</div>)}
          {result.diff && <pre className="fieldDiff">{result.diff}</pre>}
        </div>
      )}

      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={submit} disabled={running || (!selectedServer && action !== "add")}>
          {running ? "Working..." : action === "delete" ? "Delete Server" : action === "edit" ? "Save Changes" : "Add Server"}
        </button>
      </div>
    </Panel>
  );
}

function SkillWizard({ repoPath, onRefresh }: { repoPath: string; onRefresh: () => void }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [result, setResult] = useState<CreateResult | null>(null);
  const [running, setRunning] = useState(false);

  async function submit() {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<CreateResult>("create_skill", { repoPath: repoPath || null, name, description });
      setResult(res);
      if (res.ok) { setName(""); setDescription(""); onRefresh(); }
    } catch (error) {
      setResult({ ok: false, path: "", message: String(error) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <Panel title="Create Skill" subtitle="Creates a new SKILL.md scaffold in the skills directory.">
      <Field label="Skill Name (slug)">
        <input className="textInput" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-skill" />
      </Field>
      <Field label="Description">
        <input className="textInput" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="What this skill does" />
      </Field>
      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          {result.message}
          {result.path && <div className="path">{result.path}</div>}
        </div>
      )}
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={submit} disabled={running || !name}>
          {running ? "Creating..." : "Create Skill"}
        </button>
      </div>
    </Panel>
  );
}

function CommandWizard({ repoPath, onRefresh }: { repoPath: string; onRefresh: () => void }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [result, setResult] = useState<CreateResult | null>(null);
  const [running, setRunning] = useState(false);

  async function submit() {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<CreateResult>("create_command", { repoPath: repoPath || null, name, description });
      setResult(res);
      if (res.ok) { setName(""); setDescription(""); onRefresh(); }
    } catch (error) {
      setResult({ ok: false, path: "", message: String(error) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <Panel title="Create Command" subtitle="Creates a new command scaffold in the commands directory.">
      <Field label="Command Name (slug, no leading slash)">
        <input className="textInput" value={name} onChange={(e) => setName(e.target.value)} placeholder="my-command" />
      </Field>
      <Field label="Description">
        <input className="textInput" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="What this command does" />
      </Field>
      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          {result.message}
          {result.path && <div className="path">{result.path}</div>}
        </div>
      )}
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={submit} disabled={running || !name}>
          {running ? "Creating..." : "Create Command"}
        </button>
      </div>
    </Panel>
  );
}

function Packaging({ repoPath }: { repoPath: string }) {
  const [result, setResult] = useState<PackageResult | null>(null);
  const [running, setRunning] = useState<string | null>(null);

  async function run(action: "package_claude_skills" | "package_claude_extension") {
    setRunning(action);
    setResult(null);
    try {
      const res = await invoke<PackageResult>(action, { repoPath: repoPath || null });
      setResult(res);
    } catch (error) {
      setResult({ ok: false, message: String(error), artifacts: [], stdout: "", stderr: "" });
    } finally {
      setRunning(null);
    }
  }

  return (
    <Panel title="Packaging" subtitle="Build distributable packages for Claude Skills and Extensions.">
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={() => run("package_claude_skills")} disabled={!!running}>
          {running === "package_claude_skills" ? "Packaging..." : "Package Claude Skills"}
        </button>
        <button className="primaryButton" onClick={() => run("package_claude_extension")} disabled={!!running}>
          {running === "package_claude_extension" ? "Packaging..." : "Package Claude Extension"}
        </button>
      </div>
      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          <div>{result.message}</div>
          {result.artifacts.length > 0 && (
            <table>
              <thead><tr><th>Artifact</th><th>Path</th><th>Size</th></tr></thead>
              <tbody>
                {result.artifacts.map((a) => (
                  <tr key={a.name}>
                    <td>{a.name}</td>
                    <td className="path">{a.path}</td>
                    <td>{a.size} bytes</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {result.stdout && <pre className="logOutput">{result.stdout}</pre>}
          {result.stderr && <pre className="logOutput">{result.stderr}</pre>}
        </div>
      )}
    </Panel>
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
