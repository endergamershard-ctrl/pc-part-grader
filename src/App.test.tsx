import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { BenchmarkReport, HardwareInfo, HistorySummary } from "./types";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));

const hardware: HardwareInfo = {
  os: "Linux 7",
  hostname: "test-pc",
  cpu: {
    name: "Test CPU",
    physicalCores: 8,
    logicalCores: 16,
    frequencyMhz: 4200,
  },
  memory: { totalBytes: 32 * 1_073_741_824, availableBytes: 16 * 1_073_741_824 },
  gpus: [{ name: "Test GPU", vendor: "Test", backend: "Vulkan", deviceType: "DiscreteGpu" }],
  disks: [{
    name: "Test SSD",
    mountPoint: "/",
    totalBytes: 1_000_000_000_000,
    availableBytes: 500_000_000_000,
    removable: false,
  }],
};

const report: BenchmarkReport = {
  id: "run-1",
  scoringVersion: "2.0.0",
  profile: "standard",
  generatedAtUnixMs: Date.now(),
  durationMs: 120000,
  hardware,
  environment: {
    os: "Linux 7",
    cpuUsagePercent: 5,
    availableMemoryBytes: 16 * 1_073_741_824,
    totalMemoryBytes: 32 * 1_073_741_824,
    thermalCelsius: 55,
    warnings: [],
  },
  suites: [
    {
      key: "everyday",
      label: "Everyday",
      score: 1100,
      grade: 70,
      weight: 0.2,
      reliability: "high",
      bottleneck: "Bottleneck: Compression (900)",
      workloads: [
        {
          key: "compression",
          label: "Compression",
          suite: "everyday",
          unit: "MB/s",
          higherIsBetter: true,
          stats: {
            samples: [200, 210, 205],
            median: 205,
            mean: 205,
            stdDev: 5,
            coefficientOfVariation: 2.4,
            reliability: "excellent",
          },
          score: 900,
          grade: 65,
          weight: 0.25,
          valid: true,
          reliability: "excellent",
          explanation: "Median 205 MB/s",
          outputHash: "abc",
        },
      ],
    },
  ],
  components: [],
  totalScore: 1100,
  totalGrade: 70,
  letterGrade: "C",
  comparison: {
    previousTotal: 1000,
    bestTotal: 1200,
    deltaVsPrevious: 100,
    deltaVsBest: -100,
    referenceTier: "Mainstream",
    referenceNote: "Local reference values",
  },
  notes: [],
  cancelled: false,
};

const history: HistorySummary[] = [
  {
    id: "run-1",
    profile: "standard",
    generatedAtUnixMs: Date.now(),
    totalScore: 1100,
    letterGrade: "C",
    cpuName: "Test CPU",
    gpuName: "Test GPU",
    cancelled: false,
  },
];

describe("PC Part Grader v2", () => {
  afterEach(cleanup);

  beforeEach(() => {
    localStorage.clear();
    invokeMock.mockReset();
  });

  it("runs a profiled benchmark and shows suite results", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_hardware") return Promise.resolve(hardware);
      if (command === "get_community_settings") {
        return Promise.resolve({ optIn: false, endpoint: null, enabled: false });
      }
      if (command === "list_history") return Promise.resolve([]);
      if (command === "run_benchmark") return Promise.resolve(report);
      return Promise.resolve();
    });

    render(<App />);
    expect(await screen.findByText("Test CPU")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /start a benchmark/i }));
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(screen.getByRole("button", { name: /run standard benchmark/i }));
    expect(await screen.findByText(/performance report/i)).toBeInTheDocument();
    expect(screen.getByTestId("suite-everyday")).toHaveTextContent("1100");
  });

  it("shows a clear hardware detection failure", async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === "get_hardware") return Promise.reject("permission denied");
      if (command === "get_community_settings") {
        return Promise.resolve({ optIn: false, endpoint: null, enabled: false });
      }
      if (command === "list_history") return Promise.resolve([]);
      return Promise.resolve();
    });

    render(<App />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Hardware detection failed: permission denied",
      );
    });
  });

  it("opens history items", async () => {
    invokeMock.mockImplementation((command: string, args?: { id?: string }) => {
      if (command === "get_hardware") return Promise.resolve(hardware);
      if (command === "get_community_settings") {
        return Promise.resolve({ optIn: false, endpoint: null, enabled: false });
      }
      if (command === "list_history") return Promise.resolve(history);
      if (command === "load_history" && args?.id === "run-1") return Promise.resolve(report);
      return Promise.resolve();
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /^history$/i }));
    fireEvent.click(await screen.findByRole("button", { name: /^open$/i }));
    expect(await screen.findByText(/performance report/i)).toBeInTheDocument();
  });
});
