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
  sourceName: string;
  sourcePath: string;
  sourceKind: string;
  frontmatter: Record<string, string>;
  preview: string;
  installs: Record<string, string>;
};

type CommandItem = {
  name: string;
  description: string;
  argumentHint: string;
  path: string;
  sourceName: string;
  sourcePath: string;
  sourceKind: string;
  frontmatter: Record<string, string>;
  preview: string;
  installs: Record<string, string>;
};

type DesignItem = {
  name: string;
  description: string;
  path: string;
  file: string;
  sourceName: string;
  sourcePath: string;
  sourceKind: string;
  frontmatter: Record<string, string>;
  preview: string;
};

type ResourceSourceStatus = {
  name: string;
  path: string;
  resolvedPath: string;
  enabled: boolean;
  resources: string[];
  status: string;
  message: string;
};

type SourceConfigStatus = {
  path: string;
  exists: boolean;
  valid: boolean;
  error?: string;
  sources: ResourceSourceStatus[];
  warnings: string[];
};

type McpServer = {
  name: string;
  description: string;
  serverType: string;
  transport: string;
  url: string;
  command: string;
  args: string[];
  hasHeaders: boolean;
  hasEnvironment: boolean;
  enabled: boolean;
  targets: Record<string, boolean>;
  rawJson: string;
};

type McpInstallStatus = {
  tool: string;
  server: string;
  status: string;
  path: string;
  message: string;
  authStatus?: string;
  authCommand?: string;
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
  sourceConfig: SourceConfigStatus;
  registry: {
    valid: boolean;
    path: string;
    error?: string;
    servers: McpServer[];
  };
  tools: ToolStatus[];
  skills: SkillItem[];
  commands: CommandItem[];
  designs: DesignItem[];
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
  branch?: string;
  shallow: boolean;
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

type SelectiveSyncPlan = {
  title: string;
  supported: boolean;
  plans: ScriptPlan[];
  warnings: string[];
};

const views = ["Dashboard", "Skills", "Commands", "Designs", "MCP Servers", "Tools", "Diffs", "Backups", "About", "Validation", "Editor", "Logs", "Profiles"] as const;
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
  const [selectedDesign, setSelectedDesign] = useState<DesignItem | null>(null);
  const [selectedMcp, setSelectedMcp] = useState<McpServer | null>(null);
  const [structuredOutput, setStructuredOutput] = useState<StructuredOutput | null>(null);
  const [diffPreview, setDiffPreview] = useState<DiffPreview | null>(null);
  const [diffChangedOnly, setDiffChangedOnly] = useState(true);
  const [lastRefreshed, setLastRefreshed] = useState("");

  async function refresh(pathOverride = repoPath) {
    setLoading(true);
    try {
      setState(await invoke<AppState>("get_state", { repoPath: pathOverride || null }));
      setLastRefreshed(new Date().toLocaleString());
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
    const timer = window.setInterval(() => refresh(), 8000);
    return () => window.clearInterval(timer);
  }, [repoPath]);

  useEffect(() => {
    if (view === "About") loadAboutInfo();
    if (view === "Logs" || view === "Backups") loadLogs();
    if (view === "Profiles") loadProfiles();
    if (view === "Diffs" && !diffPreview) loadDiffPreview();
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

  async function runRepoImport(source: string, destination: string, branch?: string, shallow?: boolean) {
    setImportOpen(false);
    setMeta("Importing repo...");
    setOutput("Running import...");
    try {
      const result = await invoke<RepoImportResult>("run_repo_import", { source, destination, branch: branch || null, shallow: !!shallow });
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

  async function loadDiffPreview(action = "dryRunAll") {
    try {
      setDiffPreview(await invoke<DiffPreview>("preview_action", { action, repoPath: repoPath || null }));
      setMeta("Diff preview refreshed.");
    } catch (error) {
      setOutput(String(error));
      setMeta("Diff preview failed.");
    }
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setMeta("Copied to clipboard.");
    } catch (error) {
      setMeta("Copy failed.");
      setOutput(String(error));
    }
  }

  const toolById = useMemo(() => Object.fromEntries((state?.tools ?? []).map((tool: ToolStatus) => [tool.id, tool])), [state]);
  const effectiveRepoPath = state?.repo?.root ?? repoPath;

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
            {view === "Skills" && <Skills state={state} selected={selectedSkill} onSelect={setSelectedSkill} openPath={openPath} copyText={copyText} />}
            {view === "Commands" && <Commands state={state} selected={selectedCommand} onSelect={setSelectedCommand} openPath={openPath} copyText={copyText} />}
            {view === "Designs" && <Designs state={state} selected={selectedDesign} onSelect={setSelectedDesign} openPath={openPath} copyText={copyText} />}
            {view === "MCP Servers" && <Mcps state={state} selected={selectedMcp} onSelect={setSelectedMcp} onAction={reviewAction} openPath={openPath} copyText={copyText} />}
            {view === "Tools" && <Tools state={state} onAction={reviewAction} openPath={openPath} />}
            {view === "Diffs" && <Diffs preview={diffPreview} changedOnly={diffChangedOnly} onChangedOnly={setDiffChangedOnly} onPreview={loadDiffPreview} openPath={openPath} copyText={copyText} />}
            {view === "Backups" && <Backups logs={logs} onRefresh={loadLogs} openPath={openPath} copyText={copyText} />}
            {view === "About" && <About runtimeInfo={runtimeInfo} scriptVersions={scriptVersions} agentsMdInfo={agentsMdInfo} sourceConfig={state.sourceConfig} safetyWarnings={safetyWarnings} updateResult={updateResult} checkingUpdate={checkingUpdate} onCheckUpdates={checkForUpdates} onOpenAgentsMd={openAgentsMd} copyText={copyText} />}
            {view === "Validation" && <Validation onValidate={validateRepo} repoValidation={repoValidation} validating={validating} />}
            {view === "Logs" && <Logs logs={logs} onClear={clearLogs} onRefresh={loadLogs} />}
            {view === "Profiles" && <Profiles profiles={profiles} />}
            {view === "Editor" && <Editor state={state} repoPath={effectiveRepoPath} onRefresh={refresh} />}
          </>
        )}

        <section className="terminalPanel">
          <div className="panelHeader">
            <div>
              <h2>Command Output</h2>
              <p>{meta}{lastRefreshed ? ` Last refreshed: ${lastRefreshed}` : ""}</p>
            </div>
            <button className="ghostButton" onClick={() => setOutput("")}>
              Clear
            </button>
          </div>
          {structuredOutput ? <StructuredOutputView output={structuredOutput} /> : <pre>{output}</pre>}
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
      <Panel title="Health" subtitle={`${state.skills.length} skills, ${state.commands.length} commands, ${state.designs.length} designs, ${state.registry.servers.length} MCP servers.`}>
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

function Skills({
  state,
  selected,
  onSelect,
  openPath,
  copyText,
}: {
  state: AppState;
  selected: SkillItem | null;
  onSelect: (skill: SkillItem) => void;
  openPath: (path: string) => void;
  copyText: (text: string) => void;
}) {
  const current = selected ?? state.skills[0] ?? null;
  return (
    <div className="splitView">
      <Panel title="Skills" subtitle={`Read from ${state.repo?.paths.skills}${state.sourceConfig.sources.length ? ` plus ${state.sourceConfig.sources.length} configured source${state.sourceConfig.sources.length === 1 ? "" : "s"}` : ""}`}>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Source</th>
              <th>Description</th>
              <th>Claude Code</th>
              <th>Codex</th>
              <th>OpenCode</th>
            </tr>
          </thead>
          <tbody>
            {state.skills.map((skill) => (
              <tr key={skill.path} className={current?.path === skill.path ? "selectedRow" : ""} onClick={() => onSelect(skill)}>
                <td>{skill.name}</td>
                <td>
                  <StatusPill value={skill.sourceKind} />
                  <div className="path">{skill.sourceName}</div>
                </td>
                <td className="description">{skill.description || "No frontmatter description."}</td>
                <td><StatusPill value={skill.installs["claude-code"]} /></td>
                <td><StatusPill value={skill.installs.codex} /></td>
                <td><StatusPill value={skill.installs.opencode} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
      {current && (
        <Panel title="Skill Detail" subtitle={current.name}>
          <div className="detailPane">
            <Field label="Description">{current.description || "No description."}</Field>
            <Field label="Source">
              <StatusPill value={current.sourceKind} />
              <div className="path">{current.sourceName}</div>
              <div className="path">{current.sourcePath}</div>
            </Field>
            <Field label="Path"><div className="path">{current.path}</div></Field>
            <Field label="SKILL.md"><div className="path">{current.file}</div></Field>
            <Field label="Installed">
              <div className="buttonRow">
                {Object.entries(current.installs).map(([tool, status]) => <StatusPill key={tool} value={`${tool}:${status}`} />)}
              </div>
            </Field>
            <Field label="Frontmatter"><pre className="fieldDiff">{JSON.stringify(current.frontmatter, null, 2)}</pre></Field>
            <Field label="Preview"><pre className="previewText">{current.preview || "<empty>"}</pre></Field>
            <div className="buttonRow padded compact">
              <button className="ghostButton" onClick={() => openPath(current.file)}>Open SKILL.md</button>
              <button className="ghostButton" onClick={() => openPath(current.path)}>Open Folder</button>
              <button className="ghostButton" onClick={() => copyText(current.path)}>Copy Path</button>
            </div>
          </div>
        </Panel>
      )}
    </div>
  );
}

function Commands({
  state,
  selected,
  onSelect,
  openPath,
  copyText,
}: {
  state: AppState;
  selected: CommandItem | null;
  onSelect: (command: CommandItem) => void;
  openPath: (path: string) => void;
  copyText: (text: string) => void;
}) {
  const current = selected ?? state.commands[0] ?? null;
  return (
    <div className="splitView">
      <Panel title="Commands" subtitle={`Read from ${state.repo?.paths.commands}${state.sourceConfig.sources.length ? ` plus ${state.sourceConfig.sources.length} configured source${state.sourceConfig.sources.length === 1 ? "" : "s"}` : ""}`}>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Source</th>
              <th>Description</th>
              <th>Claude Code</th>
              <th>Codex Prompts</th>
              <th>OpenCode</th>
            </tr>
          </thead>
          <tbody>
            {state.commands.map((command) => (
              <tr key={command.path} className={current?.path === command.path ? "selectedRow" : ""} onClick={() => onSelect(command)}>
                <td>/{command.name}</td>
                <td>
                  <StatusPill value={command.sourceKind} />
                  <div className="path">{command.sourceName}</div>
                </td>
                <td className="description">{command.description || "No frontmatter description."}</td>
                <td><StatusPill value={command.installs["claude-code"]} /></td>
                <td><StatusPill value={command.installs.codex} /></td>
                <td><StatusPill value={command.installs.opencode} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
      {current && (
        <Panel title="Command Detail" subtitle={`/${current.name}`}>
          <div className="detailPane">
            <Field label="Description">{current.description || "No description."}</Field>
            <Field label="Argument Hint">{current.argumentHint || "None"}</Field>
            <Field label="Source">
              <StatusPill value={current.sourceKind} />
              <div className="path">{current.sourceName}</div>
              <div className="path">{current.sourcePath}</div>
            </Field>
            <Field label="Path"><div className="path">{current.path}</div></Field>
            <Field label="Installed">
              <div className="buttonRow">
                {Object.entries(current.installs).map(([tool, status]) => <StatusPill key={tool} value={`${tool}:${status}`} />)}
              </div>
            </Field>
            <Field label="Frontmatter"><pre className="fieldDiff">{JSON.stringify(current.frontmatter, null, 2)}</pre></Field>
            <Field label="Preview"><pre className="previewText">{current.preview || "<empty>"}</pre></Field>
            <div className="buttonRow padded compact">
              <button className="ghostButton" onClick={() => openPath(current.path)}>Open Command</button>
              <button className="ghostButton" onClick={() => state.repo?.paths.commands && openPath(state.repo.paths.commands)}>Open Folder</button>
              <button className="ghostButton" onClick={() => copyText(current.path)}>Copy Path</button>
            </div>
          </div>
        </Panel>
      )}
    </div>
  );
}

function Designs({
  state,
  selected,
  onSelect,
  openPath,
  copyText,
}: {
  state: AppState;
  selected: DesignItem | null;
  onSelect: (design: DesignItem) => void;
  openPath: (path: string) => void;
  copyText: (text: string) => void;
}) {
  const current = selected ?? state.designs[0] ?? null;
  return (
    <div className="splitView">
      <Panel title="Designs" subtitle={`Read from ${state.repo?.paths.designs} and root DESIGN.md${state.sourceConfig.sources.length ? ` plus ${state.sourceConfig.sources.length} configured source${state.sourceConfig.sources.length === 1 ? "" : "s"}` : ""}`}>
        {state.designs.length === 0 ? (
          <div className="empty">No DESIGN.md files found in the selected repo or configured sources.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Source</th>
                <th>Description</th>
                <th>File</th>
              </tr>
            </thead>
            <tbody>
              {state.designs.map((design) => (
                <tr key={design.path} className={current?.path === design.path ? "selectedRow" : ""} onClick={() => onSelect(design)}>
                  <td>{design.name}</td>
                  <td>
                    <StatusPill value={design.sourceKind} />
                    <div className="path">{design.sourceName}</div>
                  </td>
                  <td className="description">{design.description || "No frontmatter description."}</td>
                  <td className="path">{design.file}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>
      {current && (
        <Panel title="Design Detail" subtitle={current.name}>
          <div className="detailPane">
            <Field label="Description">{current.description || "No description."}</Field>
            <Field label="Source">
              <StatusPill value={current.sourceKind} />
              <div className="path">{current.sourceName}</div>
              <div className="path">{current.sourcePath}</div>
            </Field>
            <Field label="DESIGN.md"><div className="path">{current.file}</div></Field>
            <Field label="Frontmatter"><pre className="fieldDiff">{JSON.stringify(current.frontmatter, null, 2)}</pre></Field>
            <Field label="Preview"><pre className="previewText">{current.preview || "<empty>"}</pre></Field>
            <div className="buttonRow padded compact">
              <button className="ghostButton" onClick={() => openPath(current.file)}>Open DESIGN.md</button>
              <button className="ghostButton" onClick={() => copyText(current.path)}>Copy Path</button>
            </div>
          </div>
        </Panel>
      )}
    </div>
  );
}

function Mcps({
  state,
  selected,
  onSelect,
  onAction,
  openPath,
  copyText,
}: {
  state: AppState;
  selected: McpServer | null;
  onSelect: (server: McpServer) => void;
  onAction: (action: string) => void;
  openPath: (path: string) => void;
  copyText: (text: string) => void;
}) {
  const current = selected ?? state.registry.servers[0] ?? null;
  return (
    <>
      <div className="splitView">
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
                {state.registry.servers.map((server) => {
                  const statuses = ["claude-code", "codex", "opencode"].map((tool) => findMcp(state, tool, server.name));
                  const auth = statuses.find((status) => status?.authStatus && status.authStatus !== "not-supported")?.authStatus ?? "unknown";
                  return (
                    <tr key={server.name} className={current?.name === server.name ? "selectedRow" : ""} onClick={() => onSelect(server)}>
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
                      <td><StatusPill value={auth} /></td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </Panel>
        {current && (
          <Panel title="MCP Detail" subtitle={current.name}>
            <div className="detailPane">
              <Field label="Description">{current.description || "No description."}</Field>
              <Field label="Type / Transport">{current.serverType} / {current.transport}</Field>
              <Field label="Endpoint"><div className="path">{current.url || [current.command, ...current.args].join(" ")}</div></Field>
              <Field label="Secrets/Headers">{current.hasHeaders ? "headers present" : "no headers"}; {current.hasEnvironment ? "environment present" : "no environment"}</Field>
              <Field label="Targets">{Object.entries(current.targets).map(([target, enabled]) => `${target}:${enabled}`).join(", ") || "All/default"}</Field>
              <Field label="Install/Auth">
                <div className="statusGrid">
                  {["claude-code", "codex", "opencode"].map((tool) => {
                    const status = findMcp(state, tool, current.name);
                    return (
                      <div key={tool}>
                        <strong>{labelize(tool)}</strong>
                        <McpCell status={status} />
                        {status?.authCommand && <div className="path">{status.authCommand}</div>}
                      </div>
                    );
                  })}
                </div>
              </Field>
              <Field label="Raw JSON"><pre className="fieldDiff">{current.rawJson}</pre></Field>
              <div className="buttonRow padded compact">
                <button className="ghostButton" onClick={() => openPath(state.registry.path)}>Open Registry</button>
                <button className="ghostButton" onClick={() => copyText(current.rawJson)}>Copy Server JSON</button>
                <button className="ghostButton" onClick={() => copyText(findMcp(state, "codex", current.name)?.authCommand ?? `codex mcp login ${current.name}`)}>Copy Auth Command</button>
              </div>
            </div>
          </Panel>
        )}
      </div>
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

function Diffs({
  preview,
  changedOnly,
  onChangedOnly,
  onPreview,
  openPath,
  copyText,
}: {
  preview: DiffPreview | null;
  changedOnly: boolean;
  onChangedOnly: (value: boolean) => void;
  onPreview: (action?: string) => void;
  openPath: (path: string) => void;
  copyText: (text: string) => void;
}) {
  const sections = (preview?.sections ?? []).filter((section) => !changedOnly || section.status !== "unchanged");
  return (
    <Panel title="Diff Preview" subtitle="Inspect expected changes without opening the run dialog.">
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={() => onPreview("dryRunAll")}>Preview All</button>
        <button className="ghostButton" onClick={() => onPreview("dryRunMcps")}>Preview MCPs</button>
        <button className="ghostButton" onClick={() => onPreview("linkAgents")}>Preview Links</button>
        <button className="ghostButton" onClick={() => onChangedOnly(!changedOnly)}>{changedOnly ? "Show All" : "Changed Only"}</button>
        {preview && <button className="ghostButton" onClick={() => copyText(preview.sections.map((section) => `${section.title}\n${section.diff}`).join("\n\n"))}>Copy Diff</button>}
      </div>
      {!preview ? (
        <div className="empty">Choose a preview action.</div>
      ) : (
        <div className="previewStack standalone">
          {sections.map((section) => (
            <details className="previewSection" key={`${section.title}-${section.path}`} open={section.status === "changed" || section.status === "error"}>
              <summary>
                <span>{section.title}</span>
                <StatusPill value={section.status} />
              </summary>
              <div className="buttonRow compact">
                <button className="smallButton" onClick={() => openPath(section.path)}>Open</button>
                <button className="smallButton" onClick={() => copyText(section.diff)}>Copy</button>
              </div>
              <div className="path">{section.path}</div>
              <pre className="diffBox">{section.diff}</pre>
            </details>
          ))}
        </div>
      )}
    </Panel>
  );
}

function Backups({ logs, onRefresh, openPath, copyText }: { logs: LogEntry[]; onRefresh: () => void; openPath: (path: string) => void; copyText: (text: string) => void }) {
  const backups = logs.flatMap((log) => log.backups.map((path) => ({ path, log }))).reverse();
  return (
    <Panel title="Backups" subtitle={`${backups.length} backup paths found in recent action logs.`}>
      <div className="buttonRow padded compact">
        <button className="ghostButton" onClick={onRefresh}>Refresh</button>
      </div>
      {backups.length === 0 ? (
        <div className="empty">No backup paths have been parsed from script output yet.</div>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Timestamp</th>
              <th>Action</th>
              <th>Backup Path</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {backups.map(({ path, log }) => (
              <tr key={`${log.timestamp}-${path}`}>
                <td>{new Date(Number(log.timestamp) * 1000).toLocaleString()}</td>
                <td>{log.action}</td>
                <td className="path">{path}</td>
                <td>
                  <div className="buttonRow compact">
                    <button className="smallButton" onClick={() => openPath(path)}>Reveal</button>
                    <button className="smallButton" onClick={() => copyText(path)}>Copy</button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </Panel>
  );
}

function About({
  runtimeInfo,
  scriptVersions,
  agentsMdInfo,
  sourceConfig,
  safetyWarnings,
  updateResult,
  checkingUpdate,
  onCheckUpdates,
  onOpenAgentsMd,
  copyText,
}: {
  runtimeInfo: RuntimeInfo | null;
  scriptVersions: ScriptVersionInfo[];
  agentsMdInfo: AgentsMdInfo | null;
  sourceConfig: SourceConfigStatus;
  safetyWarnings: string[];
  updateResult: UpdateCheckResult | null;
  checkingUpdate: boolean;
  onCheckUpdates: () => void;
  onOpenAgentsMd: () => void;
  copyText: (text: string) => void;
}) {
  const diagnostics = JSON.stringify({ runtimeInfo, scriptVersions, agentsMdInfo, sourceConfig, safetyWarnings, updateResult }, null, 2);
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

      <Panel title="Resource Sources" subtitle={sourceConfig.exists ? sourceConfig.path : "No sources.json configured."}>
        {!sourceConfig.exists ? (
          <div className="empty">Create sources.json in the repo root to load skills and commands from other local source checkouts.</div>
        ) : !sourceConfig.valid ? (
          <div className="errorBox">{sourceConfig.error}</div>
        ) : sourceConfig.sources.length === 0 ? (
          <div className="empty">sources.json exists but does not list any sources.</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Status</th>
                <th>Resources</th>
                <th>Path</th>
                <th>Message</th>
              </tr>
            </thead>
            <tbody>
              {sourceConfig.sources.map((source) => (
                <tr key={`${source.name}-${source.resolvedPath}`}>
                  <td>{source.name}</td>
                  <td><StatusPill value={source.status} /></td>
                  <td>{source.resources.join(", ") || "none"}</td>
                  <td>
                    <button className="smallButton" onClick={() => copyText(source.resolvedPath)}>Copy</button>
                    <div className="path">{source.resolvedPath}</div>
                  </td>
                  <td>{source.message}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Panel>

      {sourceConfig.warnings.length > 0 && (
        <Panel title="Source Warnings" subtitle="Non-fatal issues while merging configured sources.">
          <ul className="warningList">
            {sourceConfig.warnings.map((warning) => (
              <li key={warning} className="warningItem">{warning}</li>
            ))}
          </ul>
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
          <button className="ghostButton" onClick={() => copyText(diagnostics)}>Copy Diagnostics</button>
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

type EditorTab = "MCP Editor" | "Skill Wizard" | "Command Wizard" | "Selective Sync" | "Packaging";
const editorTabs: EditorTab[] = ["MCP Editor", "Skill Wizard", "Command Wizard", "Selective Sync", "Packaging"];

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
      {tab === "Selective Sync" && <SelectiveSync repoPath={repoPath} />}
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
  const [folderName, setFolderName] = useState("");
  const [skillName, setSkillName] = useState("");
  const [description, setDescription] = useState("");
  const [body, setBody] = useState("Use this skill when...\n\n## Workflow\n\n1. Inspect the request.\n2. Apply the documented process.\n");
  const [result, setResult] = useState<CreateResult | null>(null);
  const [running, setRunning] = useState(false);

  async function submit() {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<CreateResult>("create_skill", { repoPath, folderName, skillName: skillName || folderName, description, body });
      setResult(res);
      if (res.ok) { setFolderName(""); setSkillName(""); setDescription(""); onRefresh(); }
    } catch (error) {
      setResult({ ok: false, path: "", message: String(error) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <Panel title="Create Skill" subtitle="Creates a new SKILL.md scaffold in the skills directory.">
      <Field label="Skill Name (slug)">
        <input className="textInput" value={folderName} onChange={(e) => setFolderName(e.target.value)} placeholder="my-skill" />
      </Field>
      <Field label="Display Name">
        <input className="textInput" value={skillName} onChange={(e) => setSkillName(e.target.value)} placeholder="My Skill" />
      </Field>
      <Field label="Description">
        <input className="textInput" value={description} onChange={(e) => setDescription(e.target.value)} placeholder="What this skill does" />
      </Field>
      <Field label="Body Template">
        <textarea className="textArea" value={body} onChange={(e) => setBody(e.target.value)} />
      </Field>
      <Field label="Preview">
        <pre className="fieldDiff">{`---\nname: ${skillName || folderName}\ndescription: ${description}\n---\n\n${body}`}</pre>
      </Field>
      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          {result.message}
          {result.path && <div className="path">{result.path}</div>}
        </div>
      )}
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={submit} disabled={running || !folderName || !description}>
          {running ? "Creating..." : "Create Skill"}
        </button>
      </div>
    </Panel>
  );
}

function CommandWizard({ repoPath, onRefresh }: { repoPath: string; onRefresh: () => void }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [argumentHint, setArgumentHint] = useState("");
  const [body, setBody] = useState("Describe what this command should do.\n\nInput: $ARGUMENTS\n");
  const [result, setResult] = useState<CreateResult | null>(null);
  const [running, setRunning] = useState(false);

  async function submit() {
    setRunning(true);
    setResult(null);
    try {
      const res = await invoke<CreateResult>("create_command", { repoPath, commandName: name, description, argumentHint, body });
      setResult(res);
      if (res.ok) { setName(""); setDescription(""); setArgumentHint(""); onRefresh(); }
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
      <Field label="Argument Hint">
        <input className="textInput" value={argumentHint} onChange={(e) => setArgumentHint(e.target.value)} placeholder="[topic] or [file]" />
      </Field>
      <Field label="Body Template">
        <textarea className="textArea" value={body} onChange={(e) => setBody(e.target.value)} />
      </Field>
      <Field label="Preview">
        <pre className="fieldDiff">{`---\ndescription: ${description}${argumentHint ? `\nargument-hint: ${argumentHint}` : ""}\n---\n\n${body}`}</pre>
      </Field>
      {result && (
        <div className={result.ok ? "successBox" : "errorBox"}>
          {result.message}
          {result.path && <div className="path">{result.path}</div>}
        </div>
      )}
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={submit} disabled={running || !name || !description}>
          {running ? "Creating..." : "Create Command"}
        </button>
      </div>
    </Panel>
  );
}

function SelectiveSync({ repoPath }: { repoPath: string }) {
  const [tools, setTools] = useState(["codex"]);
  const [categories, setCategories] = useState(["mcps"]);
  const [plan, setPlan] = useState<SelectiveSyncPlan | null>(null);
  const toolOptions = ["claude-code", "codex", "opencode", "github-copilot-cli"];
  const categoryOptions = ["globals", "skills", "commands", "mcps"];

  function toggle(value: string, values: string[], setter: (values: string[]) => void) {
    setter(values.includes(value) ? values.filter((item) => item !== value) : [...values, value]);
  }

  async function preview() {
    setPlan(await invoke<SelectiveSyncPlan>("plan_selective_sync", { repoPath, tools, categories }));
  }

  return (
    <Panel title="Selective Sync" subtitle="Plans narrow sync operations where the bundled scripts support them.">
      <Field label="Tools">
        <div className="buttonRow">
          {toolOptions.map((tool) => (
            <button key={tool} className={tools.includes(tool) ? "primaryButton" : "ghostButton"} onClick={() => toggle(tool, tools, setTools)}>
              {labelize(tool)}
            </button>
          ))}
        </div>
      </Field>
      <Field label="Categories">
        <div className="buttonRow">
          {categoryOptions.map((category) => (
            <button key={category} className={categories.includes(category) ? "primaryButton" : "ghostButton"} onClick={() => toggle(category, categories, setCategories)}>
              {labelize(category)}
            </button>
          ))}
        </div>
      </Field>
      <div className="buttonRow padded compact">
        <button className="primaryButton" onClick={preview}>Plan Selective Sync</button>
      </div>
      {plan && (
        <>
          {plan.warnings.length > 0 && (
            <div className="warningList">
              {plan.warnings.map((warning) => <div key={warning} className="warningItem">{warning}</div>)}
            </div>
          )}
          <div className="previewStack standalone">
            {plan.plans.map((item) => (
              <details key={`${item.action}-${item.displayCommand}`} className="previewSection" open>
                <summary>
                  <span>{item.title}</span>
                  <StatusPill value={plan.supported ? "planned" : "limited"} />
                </summary>
                <pre className="commandBox">{item.displayCommand}</pre>
                <ul>{item.affectedPaths.map((path) => <li key={path} className="path">{path}</li>)}</ul>
              </details>
            ))}
          </div>
        </>
      )}
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
  onRun: (source: string, destination: string, branch?: string, shallow?: boolean) => void;
}) {
  const [source, setSource] = useState("");
  const [destination, setDestination] = useState(defaultDestination);
  const [branch, setBranch] = useState("");
  const [shallow, setShallow] = useState(true);
  const [plan, setPlan] = useState<RepoImportPlan | null>(null);
  const [error, setError] = useState("");

  async function review() {
    setError("");
    try {
      setPlan(await invoke<RepoImportPlan>("plan_repo_import", { source, destination, branch: branch || null, shallow }));
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
                const next = event.target.value;
                setSource(next);
                if (destination === defaultDestination || destination.endsWith("/agents-import")) {
                  setDestination(deriveImportDestination(defaultDestination, next));
                }
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
          <Field label="Git Branch / Tag">
            <input
              className="textInput"
              value={branch}
              onChange={(event) => {
                setBranch(event.target.value);
                setPlan(null);
              }}
              placeholder="main, v1.0.0, or leave empty"
            />
          </Field>
          <Field label="Shallow Git Clone">
            <input type="checkbox" checked={shallow} onChange={(event) => {
              setShallow(event.target.checked);
              setPlan(null);
            }} />
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
          <button className="primaryButton" disabled={!plan} onClick={() => onRun(source, destination, branch, shallow)}>Run Import</button>
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
      {status.authStatus && <StatusPill value={status.authStatus} />}
      <div className="path">{status.message}</div>
    </>
  );
}

function StructuredOutputView({ output }: { output: StructuredOutput }) {
  const groups: Array<[string, string[]]> = [
    ["Summary", output.summary],
    ["Changed", output.changed],
    ["Skipped", output.skipped],
    ["Warnings", output.warnings],
    ["Errors", output.errors],
    ["Backups", output.backups],
    ["Auth Hints", output.authHints],
  ];
  return (
    <div className="structuredOutput">
      <div className="structuredSummary">
        <StatusPill value={output.exitCode === 0 || output.exitCode === undefined ? "ok" : "error"} />
        <span className="path">exit: {output.exitCode ?? "unknown"}</span>
      </div>
      {groups.map(([label, lines]) => lines.length > 0 && (
        <details key={label} open={label === "Summary" || label === "Errors" || label === "Warnings"}>
          <summary>{label} ({lines.length})</summary>
          <pre>{lines.join("\n")}</pre>
        </details>
      ))}
      <details>
        <summary>Raw Output</summary>
        <pre>{["stdout:", output.rawStdout || "<empty>", "", "stderr:", output.rawStderr || "<empty>"].join("\n")}</pre>
      </details>
    </div>
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

function deriveImportDestination(defaultDestination: string, source: string) {
  const base = defaultDestination.replace(/\/agents-import$/, "") || defaultDestination;
  const clean = source.split("?")[0].replace(/\/$/, "");
  const last = clean.split("/").pop() || "agents-import";
  const name = last.replace(/\.git$/, "").replace(/\.zip$/, "") || "agents-import";
  return `${base}/${name}`;
}

createRoot(document.getElementById("root")!).render(<App />);
