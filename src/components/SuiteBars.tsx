import type { SuiteScore } from "../types";

export function SuiteBars({ suites }: { suites: SuiteScore[] }) {
  const max = Math.max(1000, ...suites.map((suite) => suite.score ?? 0));
  return (
    <div className="suite-bars" role="img" aria-label="Suite score chart">
      {suites.map((suite) => {
        const value = suite.score ?? 0;
        const width = suite.score == null ? 0 : Math.min(100, (value / max) * 100);
        return (
          <div className="suite-bar-row" key={suite.key}>
            <span>{suite.label}</span>
            <div className="suite-bar-track">
              <span style={{ width: `${width}%` }} />
            </div>
            <strong>{suite.score == null ? "—" : Math.round(suite.score)}</strong>
          </div>
        );
      })}
    </div>
  );
}
