import { type FormEvent, useMemo, useState } from "react";

import {
  getActionTestResult,
  listActionTests,
  startActionTest,
  watchActionTest,
  type Catalog,
  type SessionPlan,
  type TestResult,
} from "./api";

type RunStatus = "idle" | "planning" | "planned" | "starting" | "polling" | "complete";

const POLL_INTERVAL_MS = 3_000;

function describeError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "The desktop command failed. Check the repository and try again.";
}

export default function App() {
  const [repoRoot, setRepoRoot] = useState("");
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [selectedTest, setSelectedTest] = useState("");
  const [plan, setPlan] = useState<SessionPlan | null>(null);
  const [result, setResult] = useState<TestResult | null>(null);
  const [status, setStatus] = useState<RunStatus>("idle");
  const [error, setError] = useState("");
  const [copiedCommand, setCopiedCommand] = useState("");

  const unsupportedActions = useMemo(
    () => catalog?.actions.filter((action) => !action.supported) ?? [],
    [catalog],
  );

  const normalizedRoot = repoRoot.trim();
  const busy = status === "planning" || status === "starting" || status === "polling";

  async function refresh(event?: FormEvent) {
    event?.preventDefault();
    if (!normalizedRoot) {
      setError("Enter a repository path first.");
      return;
    }

    setError("");
    try {
      const nextCatalog = await listActionTests(normalizedRoot);
      setCatalog(nextCatalog);
      setSelectedTest((current) =>
        nextCatalog.tests.some((test) => test.name === current)
          ? current
          : (nextCatalog.tests[0]?.name ?? ""),
      );
      setPlan(null);
      setResult(null);
      setStatus("idle");
    } catch (commandError) {
      setError(describeError(commandError));
    }
  }

  async function previewRun() {
    if (!selectedTest) {
      setError("Select an Action Test first.");
      return;
    }

    setStatus("planning");
    setError("");
    setResult(null);
    try {
      const response = await startActionTest(
        normalizedRoot,
        selectedTest,
        false,
      );
      setPlan(response.plan);
      setStatus("planned");
    } catch (commandError) {
      setStatus("idle");
      setError(describeError(commandError));
    }
  }

  async function confirmRun() {
    if (!plan) {
      return;
    }

    setStatus("starting");
    setError("");
    try {
      const response = await startActionTest(
        normalizedRoot,
        selectedTest,
        true,
      );
      setPlan(response.plan);
      if (response.result) {
        finish(response.result);
      } else {
        await pollForResult(response.plan.session_id);
      }
    } catch (commandError) {
      setStatus("planned");
      setError(describeError(commandError));
    }
  }

  async function pollForResult(sessionId: string): Promise<void> {
    setStatus("polling");
    try {
      const stored = await getActionTestResult(normalizedRoot, sessionId);
      if (stored) {
        finish(stored);
        return;
      }

      const response = await watchActionTest(normalizedRoot, sessionId);
      if (response.result) {
        finish(response.result);
        return;
      }
      if (response.pending) {
        window.setTimeout(() => void pollForResult(sessionId), POLL_INTERVAL_MS);
        return;
      }

      throw new Error("The test stopped without a result.");
    } catch (commandError) {
      setStatus("planned");
      setError(describeError(commandError));
    }
  }

  function finish(nextResult: TestResult) {
    setResult(nextResult);
    setStatus("complete");
  }

  async function copyCommand(command: string) {
    try {
      await navigator.clipboard.writeText(command);
      setCopiedCommand(command);
      window.setTimeout(() => setCopiedCommand(""), 1_500);
    } catch {
      setError("Could not copy the command. Select it and copy manually.");
    }
  }

  return (
    <main className="shell">
      <header className="hero">
        <div>
          <p className="eyebrow">GitHub Workflow Workbench</p>
          <h1>Action Tests</h1>
          <p className="lede">
            Discover composite actions, review the generated remote run, and
            inspect its evidence.
          </p>
        </div>
        <span className={`status status-${status}`}>
          {status === "idle" && "Ready"}
          {status === "planning" && "Planning"}
          {status === "planned" && "Awaiting confirmation"}
          {status === "starting" && "Starting remote run"}
          {status === "polling" && "Watching remote run"}
          {status === "complete" && (result?.passed ? "Passed" : "Failed")}
        </span>
      </header>

      <section className="panel repository-panel" aria-labelledby="repository-heading">
        <div className="section-heading">
          <div>
            <p className="step-label">Step 1</p>
            <h2 id="repository-heading">Choose a repository</h2>
          </div>
          {catalog && (
            <span className="count">
              {catalog.actions.length} actions · {catalog.tests.length} tests
            </span>
          )}
        </div>
        <form className="repo-form" onSubmit={refresh}>
          <label htmlFor="repo-root">Repository path</label>
          <div className="input-row">
            <input
              id="repo-root"
              value={repoRoot}
              onChange={(event) => setRepoRoot(event.target.value)}
              placeholder="/home/me/projects/example-action"
              autoComplete="off"
              spellCheck={false}
            />
            <button className="button button-secondary" disabled={busy} type="submit">
              {catalog ? "Refresh" : "List Action Tests"}
            </button>
          </div>
        </form>
      </section>

      {error && (
        <div className="error-banner" role="alert">
          <strong>Could not continue</strong>
          <pre>{error}</pre>
        </div>
      )}

      {catalog && (
        <div className="content-grid">
          <section className="panel" aria-labelledby="tests-heading">
            <div className="section-heading">
              <div>
                <p className="step-label">Step 2</p>
                <h2 id="tests-heading">Select a test</h2>
              </div>
            </div>

            {catalog.tests.length === 0 ? (
              <p className="empty-state">
                No tests found in <code>.github-workbench/tests</code>.
              </p>
            ) : (
              <div className="test-list">
                {catalog.tests.map((test) => (
                  <label
                    className={`test-card ${
                      selectedTest === test.name ? "test-card-selected" : ""
                    }`}
                    key={test.path}
                  >
                    <input
                      type="radio"
                      name="action-test"
                      value={test.name}
                      checked={selectedTest === test.name}
                      onChange={() => {
                        setSelectedTest(test.name);
                        setPlan(null);
                        setResult(null);
                        setStatus("idle");
                      }}
                    />
                    <span>
                      <strong>{test.name}</strong>
                      <small>{test.path}</small>
                    </span>
                  </label>
                ))}
              </div>
            )}

            <button
              className="button button-primary full-width"
              type="button"
              disabled={!selectedTest || busy}
              onClick={() => void previewRun()}
            >
              Review run plan
            </button>

            <div className="action-summary">
              <h3>Discovered actions</h3>
              {catalog.actions.map((action) => (
                <div className="action-row" key={action.definition.manifest_path}>
                  <span>
                    <strong>{action.definition.name}</strong>
                    <small>{action.definition.manifest_path}</small>
                  </span>
                  <span className={action.supported ? "tag tag-ok" : "tag tag-warning"}>
                    {action.supported ? "Composite" : "Unsupported"}
                  </span>
                </div>
              ))}
              {unsupportedActions.map((action) => (
                <p className="warning" key={`${action.definition.manifest_path}-warning`}>
                  {action.warning}
                </p>
              ))}
            </div>
          </section>

          <section className="panel run-panel" aria-labelledby="run-heading">
            <div className="section-heading">
              <div>
                <p className="step-label">Step 3</p>
                <h2 id="run-heading">Review and run</h2>
              </div>
            </div>

            {!plan ? (
              <p className="empty-state">
                Select a test and review its plan before starting a remote run.
              </p>
            ) : (
              <>
                <div className="plan-summary">
                  <h3>{plan.git_plan.summary}</h3>
                  <dl>
                    <div>
                      <dt>Workflow</dt>
                      <dd>{plan.workflow_path}</dd>
                    </div>
                    <div>
                      <dt>Remote ref</dt>
                      <dd>
                        {plan.cleanup_identity.remote}/{plan.cleanup_identity.ref_name}
                      </dd>
                    </div>
                    <div>
                      <dt>Session</dt>
                      <dd>{plan.session_id}</dd>
                    </div>
                  </dl>
                  <h4>Preconditions</h4>
                  <ul>
                    {plan.git_plan.preconditions.map((precondition) => (
                      <li key={precondition}>{precondition}</li>
                    ))}
                  </ul>
                </div>

                {!result && (
                  <button
                    className="button button-danger full-width"
                    type="button"
                    disabled={busy}
                    onClick={() => void confirmRun()}
                  >
                    {status === "starting" || status === "polling"
                      ? "Remote run in progress…"
                      : "Confirm & start remote test"}
                  </button>
                )}
              </>
            )}

            {result && (
              <div className={`result result-${result.passed ? "pass" : "fail"}`}>
                <p className="result-kicker">
                  {result.passed ? "Assertions passed" : "Assertions failed"}
                </p>
                <h3>{result.conclusion}</h3>
                <dl>
                  <div>
                    <dt>Run</dt>
                    <dd>
                      <a href={result.run_url} target="_blank" rel="noreferrer">
                        GitHub Actions #{result.run_id}
                      </a>
                    </dd>
                  </div>
                  <div>
                    <dt>Manifest</dt>
                    <dd>{result.manifest_path ?? "Not produced"}</dd>
                  </div>
                  <div>
                    <dt>Logs</dt>
                    <dd>{result.logs_path}</dd>
                  </div>
                </dl>
              </div>
            )}
          </section>
        </div>
      )}

      <section className="panel cleanup-panel" aria-labelledby="cleanup-heading">
        <div>
          <p className="step-label">Cleanup stays in the CLI</p>
          <h2 id="cleanup-heading">Remove temporary refs safely</h2>
          <p>
            Inspect queued cleanup work, then run the specific item after checking
            its identity.
          </p>
        </div>
        <div className="command-list">
          {["gww cleanup list", "gww cleanup run <item-id>"].map((command) => (
            <div className="command" key={command}>
              <code>{command}</code>
              <button
                type="button"
                onClick={() => void copyCommand(command)}
                aria-label={`Copy ${command}`}
              >
                {copiedCommand === command ? "Copied" : "Copy"}
              </button>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
