export type BenchmarkProfile = "standard" | "extended";

export interface CpuInfo {
  name: string;
  physicalCores: number;
  logicalCores: number;
  frequencyMhz: number;
}

export interface MemoryInfo {
  totalBytes: number;
  availableBytes: number;
}

export interface GpuInfo {
  name: string;
  vendor: string;
  backend: string;
  deviceType: string;
}

export interface DiskInfo {
  name: string;
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
  removable: boolean;
}

export interface HardwareInfo {
  os: string;
  hostname: string;
  cpu: CpuInfo;
  memory: MemoryInfo;
  gpus: GpuInfo[];
  disks: DiskInfo[];
}

export interface RunEnvironment {
  os: string;
  cpuUsagePercent: number;
  availableMemoryBytes: number;
  totalMemoryBytes: number;
  thermalCelsius: number | null;
  warnings: string[];
}

export interface SampleStats {
  samples: number[];
  median: number;
  mean: number;
  stdDev: number;
  coefficientOfVariation: number;
  reliability: string;
}

export interface WorkloadResult {
  key: string;
  label: string;
  suite: string;
  unit: string;
  higherIsBetter: boolean;
  stats: SampleStats | null;
  score: number | null;
  grade: number | null;
  weight: number;
  valid: boolean;
  reliability: string;
  explanation: string;
  outputHash: string | null;
}

export interface SuiteScore {
  key: string;
  label: string;
  score: number | null;
  grade: number | null;
  weight: number;
  reliability: string;
  bottleneck: string | null;
  workloads: WorkloadResult[];
}

export interface ComponentScore {
  key: string;
  label: string;
  score: number | null;
  weight: number;
  confidence: string;
  explanation: string;
}

export interface LocalComparison {
  previousTotal: number | null;
  bestTotal: number | null;
  deltaVsPrevious: number | null;
  deltaVsBest: number | null;
  referenceTier: string | null;
  referenceNote: string;
}

export interface BenchmarkReport {
  id: string;
  scoringVersion: string;
  profile: BenchmarkProfile;
  generatedAtUnixMs: number;
  durationMs: number;
  hardware: HardwareInfo;
  environment: RunEnvironment;
  suites: SuiteScore[];
  components: ComponentScore[];
  totalScore: number | null;
  totalGrade: number | null;
  letterGrade: string;
  comparison: LocalComparison;
  notes: string[];
  cancelled: boolean;
}

export interface HistorySummary {
  id: string;
  profile: BenchmarkProfile;
  generatedAtUnixMs: number;
  totalScore: number | null;
  letterGrade: string;
  cpuName: string;
  gpuName: string;
  cancelled: boolean;
}

export interface BenchmarkProgress {
  stage: string;
  suite: string;
  workload: string;
  percent: number;
  message: string;
  elapsedMs: number;
  estimatedRemainingMs: number | null;
}

export interface CommunitySettings {
  optIn: boolean;
  endpoint: string | null;
  enabled: boolean;
}

export type AppPage = "home" | "run" | "results" | "history" | "settings";
