import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";
import { SuiteBars } from "./components/SuiteBars";
import type {
  AppPage,
  BenchmarkProfile,
  BenchmarkProgress,
  BenchmarkReport,
  CommunitySettings,
  HardwareInfo,
  HistorySummary,
} from "./types";

function bytes(value: number) {
  if (!value) return "Unknown";
  return `${(value / 1_073_741_824).toFixed(1)} GiB`;
}

function formatDuration(ms: number) {
  const total = Math.max(0, Math.round(ms / 1000));
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`;
}

function deltaText(value: number | null | undefined) {
  if (value == null) return "—";
  const sign = value > 0 ? "+" : "";
  return `${sign}${Math.round(value)}`;
}

export default function App() {
  const [page, setPage] = useState<AppPage>("home");
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [report, setReport] = useState<BenchmarkReport | null>(null);
  const [history, setHistory] = useState<HistorySummary[]>([]);
  const [profile, setProfile] = useState<BenchmarkProfile>("standard");
  const [consent, setConsent] = useState(false);
  const [running, setRunning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<BenchmarkProgress>({
    stage: "ready",
    suite: "system",
    workload: "Ready",
    percent: 0,
    message: "Ready to benchmark",
    elapsedMs: 0,
    estimatedRemainingMs: null,
  });
  const [community, setCommunity] = useState<CommunitySettings>({
    optIn: false,
    endpoint: null,
    enabled: false,
  });
  const [communityPreview, setCommunityPreview] = useState<string | null>(null);

  const estimatedLabel = profile === "extended" ? "about 10–15 minutes" : "about 3–5 minutes";

  async function refreshHistory() {
    try {
      setHistory(await invoke<HistorySummary[]>("list_history"));
    } catch {
      setHistory([]);
    }
  }

  useEffect(() => {
    invoke<HardwareInfo>("get_hardware")
      .then(setHardware)
      .catch((reason) => setError(`Hardware detection failed: ${String(reason)}`))
      .finally(() => setLoading(false));
    invoke<CommunitySettings>("get_community_settings")
      .then(setCommunity)
      .catch(() => undefined);
    void refreshHistory();

    const unlisten = listen<BenchmarkProgress>("benchmark-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  const bottleneck = useMemo(() => {
    if (!report) return null;
    return report.suites
      .flatMap((suite) => suite.workloads)
      .filter((workload) => workload.valid && workload.score != null)
      .sort((a, b) => (a.score ?? 0) - (b.score ?? 0))[0];
  }, [report]);

  async function startBenchmark() {
    if (!consent || running) return;
    setError(null);
    setRunning(true);
    setPage("run");
    setProgress({
      stage: "starting",
      suite: "system",
      workload: "Preparing",
      percent: 1,
      message: "Preparing benchmark",
      elapsedMs: 0,
      estimatedRemainingMs: profile === "extended" ? 780_000 : 240_000,
    });
    try {
      const next = await invoke<BenchmarkReport>("run_benchmark", { profile });
      setReport(next);
      setHardware(next.hardware);
      await refreshHistory();
      setPage("results");
    } catch (reason) {
      setError(`Benchmark failed: ${String(reason)}`);
    } finally {
      setRunning(false);
    }
  }

  async function cancelBenchmark() {
    try {
      await invoke("cancel_benchmark");
    } catch (reason) {
      setError(`Could not cancel benchmark: ${String(reason)}`);
    }
  }

  async function openHistoryItem(id: string) {
    try {
      const loaded = await invoke<BenchmarkReport>("load_history", { id });
      setReport(loaded);
      setPage("results");
    } catch (reason) {
      setError(`Could not open history item: ${String(reason)}`);
    }
  }

  async function removeHistoryItem(id: string) {
    try {
      await invoke("delete_history", { id });
      if (report?.id === id) setReport(null);
      await refreshHistory();
    } catch (reason) {
      setError(`Could not delete history item: ${String(reason)}`);
    }
  }

  async function exportReport() {
    if (!report) return;
    try {
      const json = await invoke<string>("export_report_json", { id: report.id });
      const blob = new Blob([json], { type: "application/json" });
      const link = document.createElement("a");
      link.href = URL.createObjectURL(blob);
      link.download = `pc-part-grader-${report.id}.json`;
      link.click();
      URL.revokeObjectURL(link.href);
    } catch (reason) {
      setError(`Export failed: ${String(reason)}`);
    }
  }

  async function saveCommunity() {
    try {
      const saved = await invoke<CommunitySettings>("set_community_settings", {
        settings: community,
      });
      setCommunity(saved);
      setCommunityPreview(
        saved.enabled
          ? "Community endpoint saved. Uploads remain disabled until a live service is available."
          : "Community sharing stays off until you opt in and configure an endpoint.",
      );
    } catch (reason) {
      setError(`Could not save community settings: ${String(reason)}`);
    }
  }

  async function previewCommunity() {
    if (!report) return;
    try {
      const payload = await invoke("preview_community_payload", { id: report.id });
      setCommunityPreview(JSON.stringify(payload, null, 2));
    } catch (reason) {
      setCommunityPreview(String(reason));
    }
  }

  return (
    <main className="app-shell">
      <header className="app-header">
        <a className="brand" href="#top" onClick={() => setPage("home")}>
          <span className="brand-mark">A+</span>
          <span>
            <strong>PC Part Grader</strong>
            <small>PCMark-inspired system benchmark</small>
          </span>
        </a>
        <nav className="nav-tabs" aria-label="Primary">
          {(
            [
              ["home", "Home"],
              ["run", "Run"],
              ["results", "Results"],
              ["history", "History"],
              ["settings", "Settings"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              className={page === id ? "nav-tab nav-tab--active" : "nav-tab"}
              onClick={() => setPage(id)}
            >
              {label}
            </button>
          ))}
        </nav>
      </header>

      {error && (
        <div className="alert" role="alert">
          <strong>Something went wrong</strong>
          <span>{error}</span>
        </div>
      )}

      {page === "home" && (
        <section className="hero" id="top">
          <div className="hero-copy">
            <span className="eyebrow">Know what your PC can do</span>
            <h1>Real workloads. Transparent scores.</h1>
            <p>
              Run Everyday, Productivity, Content Creation, Graphics, and Storage suites with
              warm-ups, repeated samples, and reliability checks — then compare against your own
              history.
            </p>
            <div className="system-chip">
              <span className={loading ? "status-dot status-dot--pulse" : "status-dot"} />
              {loading
                ? "Detecting this PC…"
                : hardware
                  ? `${hardware.hostname} · ${hardware.os}`
                  : "Hardware unavailable"}
            </div>
            <div className="hero-actions">
              <button className="button button--primary" onClick={() => setPage("run")}>
                Start a benchmark
              </button>
              <button className="button button--ghost" onClick={() => setPage("history")}>
                View history
              </button>
            </div>
          </div>
          <div className="total-card">
            <span>Latest score</span>
            <strong>
              {report?.totalScore == null ? "—" : Math.round(report.totalScore)}
            </strong>
            <small>
              {report
                ? `${report.letterGrade} · model v${report.scoringVersion}`
                : "No runs yet"}
            </small>
          </div>
        </section>
      )}

      {page === "home" && hardware && (
        <section className="section">
          <div className="section-heading">
            <div>
              <span className="eyebrow">System overview</span>
              <h2>Detected hardware</h2>
            </div>
          </div>
          <div className="hardware-grid">
            <article className="hardware-card">
              <span className="hardware-icon">CPU</span>
              <div>
                <small>Processor</small>
                <strong>{hardware.cpu.name}</strong>
                <p>
                  {hardware.cpu.physicalCores} cores · {hardware.cpu.logicalCores} threads
                </p>
              </div>
            </article>
            <article className="hardware-card">
              <span className="hardware-icon">GPU</span>
              <div>
                <small>Graphics</small>
                <strong>{hardware.gpus[0]?.name ?? "No adapter found"}</strong>
                <p>
                  {hardware.gpus[0]
                    ? `${hardware.gpus[0].vendor} · ${hardware.gpus[0].backend}`
                    : "GPU suite may be unavailable"}
                </p>
              </div>
            </article>
            <article className="hardware-card">
              <span className="hardware-icon">RAM</span>
              <div>
                <small>Memory</small>
                <strong>{bytes(hardware.memory.totalBytes)}</strong>
                <p>{bytes(hardware.memory.availableBytes)} available</p>
              </div>
            </article>
            <article className="hardware-card">
              <span className="hardware-icon">SSD</span>
              <div>
                <small>Primary storage</small>
                <strong>
                  {hardware.disks.find((d) => d.mountPoint === "/")?.name ||
                    hardware.disks[0]?.name ||
                    "Unknown"}
                </strong>
                <p>
                  {hardware.disks[0]
                    ? `${bytes(hardware.disks[0].totalBytes)} total`
                    : "Unavailable"}
                </p>
              </div>
            </article>
          </div>
        </section>
      )}

      {page === "run" && (
        <section className="benchmark-panel section-panel">
          <div>
            <span className="eyebrow">Benchmark runner</span>
            <h2>{running ? progress.message : "Choose a profile"}</h2>
            <p>
              Standard is best for everyday checks. Extended uses more samples and larger data sets
              for more stable results.
            </p>
            <div className="profile-grid">
              <button
                className={
                  profile === "standard" ? "profile-card profile-card--active" : "profile-card"
                }
                disabled={running}
                onClick={() => setProfile("standard")}
              >
                <strong>Standard</strong>
                <span>About 3–5 minutes · 3 samples</span>
              </button>
              <button
                className={
                  profile === "extended" ? "profile-card profile-card--active" : "profile-card"
                }
                disabled={running}
                onClick={() => setProfile("extended")}
              >
                <strong>Extended</strong>
                <span>About 10–15 minutes · 7 samples</span>
              </button>
            </div>
            <ul className="checklist">
              <li>Plug in power and close heavy apps</li>
              <li>Leave the machine alone during the run</li>
              <li>Temporary storage files are removed automatically</li>
              <li>Estimated duration: {estimatedLabel}</li>
            </ul>
          </div>
          <div className="run-controls">
            {running ? (
              <>
                <div className="progress-label">
                  <span>
                    {progress.suite} · {progress.workload}
                  </span>
                  <strong>{progress.percent}%</strong>
                </div>
                <div
                  className="progress-track"
                  role="progressbar"
                  aria-valuenow={progress.percent}
                  aria-valuemin={0}
                  aria-valuemax={100}
                >
                  <span style={{ width: `${progress.percent}%` }} />
                </div>
                <div className="progress-meta">
                  <span>Elapsed {formatDuration(progress.elapsedMs)}</span>
                  <span>
                    Remaining{" "}
                    {progress.estimatedRemainingMs == null
                      ? "—"
                      : formatDuration(progress.estimatedRemainingMs)}
                  </span>
                </div>
                <button className="button button--secondary" onClick={cancelBenchmark}>
                  Cancel safely
                </button>
              </>
            ) : (
              <>
                <label className="consent">
                  <input
                    type="checkbox"
                    checked={consent}
                    onChange={(event) => setConsent(event.currentTarget.checked)}
                  />
                  <span>I understand this will load my PC for {estimatedLabel}.</span>
                </label>
                <button
                  className="button button--primary"
                  disabled={!consent || loading || !hardware}
                  onClick={startBenchmark}
                >
                  Run {profile} benchmark
                </button>
              </>
            )}
          </div>
        </section>
      )}

      {page === "results" && report && (
        <section className="section results" aria-live="polite">
          <div className="section-heading">
            <div>
              <span className="eyebrow">Your results</span>
              <h2>
                {report.cancelled ? "Partial report" : "Performance report"} · {report.letterGrade}
              </h2>
            </div>
            <button className="button button--ghost" onClick={exportReport}>
              Export JSON
            </button>
          </div>

          <div className="results-hero">
            <div className="total-card total-card--inline">
              <span>Overall score</span>
              <strong>
                {report.totalScore == null ? "—" : Math.round(report.totalScore)}
              </strong>
              <small>
                Grade {report.totalGrade == null ? "—" : Math.round(report.totalGrade)} / 100 ·{" "}
                {report.comparison.referenceTier ?? "Unranked tier"}
              </small>
            </div>
            <div className="comparison-card">
              <h3>Comparisons</h3>
              <p>
                vs previous: <strong>{deltaText(report.comparison.deltaVsPrevious)}</strong>
              </p>
              <p>
                vs best: <strong>{deltaText(report.comparison.deltaVsBest)}</strong>
              </p>
              <small>{report.comparison.referenceNote}</small>
            </div>
          </div>

          <SuiteBars suites={report.suites} />

          <div className="scores-grid">
            {report.suites.map((suite) => (
              <article className="score-card" key={suite.key} data-testid={`suite-${suite.key}`}>
                <div className="score-card__top">
                  <div>
                    <span className="eyebrow">{suite.label}</span>
                    <strong className="score-value">
                      {suite.score == null ? "—" : Math.round(suite.score)}
                    </strong>
                  </div>
                  <span className={`confidence confidence--${suite.reliability}`}>
                    {suite.reliability}
                  </span>
                </div>
                <p>{suite.bottleneck ?? "All measured workloads contributed."}</p>
                <ul className="workload-list">
                  {suite.workloads.map((workload) => (
                    <li key={workload.key}>
                      <span>{workload.label}</span>
                      <strong>
                        {workload.score == null ? "—" : Math.round(workload.score)} ·{" "}
                        {workload.reliability}
                      </strong>
                    </li>
                  ))}
                </ul>
              </article>
            ))}
          </div>

          {bottleneck && (
            <div className="alert alert--info">
              <strong>Biggest bottleneck</strong>
              <span>
                {bottleneck.label} scored {Math.round(bottleneck.score ?? 0)} (
                {bottleneck.explanation})
              </span>
            </div>
          )}

          {report.environment.warnings.length > 0 && (
            <div className="alert" role="status">
              <strong>Run conditions</strong>
              <span>{report.environment.warnings.join(" ")}</span>
            </div>
          )}

          <details className="details">
            <summary>Raw notes and environment</summary>
            <div className="details-content">
              <pre>{JSON.stringify(report.environment, null, 2)}</pre>
              <ul>
                {report.notes.length ? (
                  report.notes.map((note) => <li key={note}>{note}</li>)
                ) : (
                  <li>All planned suites completed.</li>
                )}
              </ul>
            </div>
          </details>
        </section>
      )}

      {page === "results" && !report && (
        <section className="section">
          <div className="empty-state">No results yet. Run a benchmark to generate a report.</div>
        </section>
      )}

      {page === "history" && (
        <section className="section">
          <div className="section-heading">
            <div>
              <span className="eyebrow">Run history</span>
              <h2>Saved locally on this PC</h2>
            </div>
          </div>
          {history.length === 0 ? (
            <div className="empty-state">No saved runs yet.</div>
          ) : (
            <div className="history-list">
              {history.map((item) => (
                <article className="history-card" key={item.id}>
                  <div>
                    <strong>
                      {item.totalScore == null ? "—" : Math.round(item.totalScore)} ·{" "}
                      {item.letterGrade}
                    </strong>
                    <p>
                      {item.profile} · {new Date(item.generatedAtUnixMs).toLocaleString()}
                    </p>
                    <small>
                      {item.cpuName} · {item.gpuName}
                    </small>
                  </div>
                  <div className="history-actions">
                    <button className="button button--ghost" onClick={() => openHistoryItem(item.id)}>
                      Open
                    </button>
                    <button
                      className="button button--secondary"
                      onClick={() => removeHistoryItem(item.id)}
                    >
                      Delete
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
          {history.length > 1 && (
            <div className="trend-card">
              <h3>Score trend</h3>
              <svg viewBox="0 0 320 80" role="img" aria-label="Score history trend">
                <polyline
                  fill="none"
                  stroke="#a3ff47"
                  strokeWidth="2"
                  points={history
                    .slice()
                    .reverse()
                    .map((item, index, arr) => {
                      const x = arr.length === 1 ? 0 : (index / (arr.length - 1)) * 320;
                      const score = item.totalScore ?? 0;
                      const max = Math.max(...arr.map((h) => h.totalScore ?? 0), 1);
                      const y = 70 - (score / max) * 60;
                      return `${x},${y}`;
                    })
                    .join(" ")}
                />
              </svg>
            </div>
          )}
        </section>
      )}

      {page === "settings" && (
        <section className="section">
          <div className="section-heading">
            <div>
              <span className="eyebrow">Settings</span>
              <h2>Privacy and community</h2>
            </div>
          </div>
          <div className="settings-card">
            <label className="consent">
              <input
                type="checkbox"
                checked={community.optIn}
                onChange={(event) =>
                  setCommunity((current) => ({
                    ...current,
                    optIn: event.currentTarget.checked,
                  }))
                }
              />
              <span>Opt in to future community comparisons (uploads stay off until a service exists).</span>
            </label>
            <label className="field">
              <span>Community endpoint</span>
              <input
                value={community.endpoint ?? ""}
                placeholder="https://example.invalid/api (optional, disabled by default)"
                onChange={(event) =>
                  setCommunity((current) => ({
                    ...current,
                    endpoint: event.currentTarget.value || null,
                  }))
                }
              />
            </label>
            <div className="hero-actions">
              <button className="button button--primary" onClick={saveCommunity}>
                Save settings
              </button>
              <button className="button button--ghost" onClick={previewCommunity} disabled={!report}>
                Preview redacted payload
              </button>
            </div>
            {communityPreview && <pre className="preview-box">{communityPreview}</pre>}
          </div>
        </section>
      )}

      <footer>
        <span>PC Part Grader · scoring model 2.0</span>
        <span>Independent benchmarks inspired by PCMark-style suites — not affiliated with UL.</span>
      </footer>
    </main>
  );
}
