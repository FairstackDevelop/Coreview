import { invoke } from "@tauri-apps/api/core";

export interface MemModule { size: string; kind: string; speed: string; manufacturer: string }
export interface GpuInfo { name: string; vendor: string; vram: string; cores: string; driver: string; displays: string[] }
export interface DiskInfo { name: string; mount: string; fs: string; kind: string; total: number; available: number; removable: boolean }
export interface PhysDisk { name: string; media: string; size: number; health: string; protocol: string }
export interface NetIface { name: string; mac: string; ips: string[]; mtu: number; state: string }
export interface BatteryInfo {
  percent: number;
  charging: boolean;
  plugged: boolean;
  cycleCount: number | null;
  healthPercent: number | null;
  designMah: number | null;
  maxMah: number | null;
}

export interface HardwareInfo {
  os: { name: string; version: string; kernel: string; arch: string; hostname: string; uptimeSecs: number };
  cpu: { brand: string; vendor: string; physicalCores: number; logicalCores: number; baseMhz: number };
  memory: { total: number; swapTotal: number; modules: MemModule[] };
  board: { manufacturer: string; model: string; serial: string; firmware: string };
  gpus: GpuInfo[];
  disks: DiskInfo[];
  physicalDisks: PhysDisk[];
  network: NetIface[];
  battery: BatteryInfo | null;
}

export interface Temp { label: string; celsius: number; max: number | null; critical: number | null }

export interface LiveStats {
  timestamp: number;
  cpuTotal: number;
  cpuCores: number[];
  cpuMhz: number;
  memUsed: number;
  memTotal: number;
  swapUsed: number;
  swapTotal: number;
  temps: Temp[];
  netRx: number;
  netTx: number;
  diskRead: number;
  diskWrite: number;
  disks: { mount: string; total: number; available: number }[];
  load: [number, number, number];
}

export interface PowerStats {
  watts: number | null;
  batteryWatts: number | null;
  adapterWatts: number | null;
  adapterRated: number | null;
  onAc: boolean;
  percent: number | null;
  charging: boolean;
  minutesRemaining: number | null;
  cpuWatts: number | null;
  gpuWatts: number | null;
  aneWatts: number | null;
  dramWatts: number | null;
  batteryVolts: number | null;
  batteryAmps: number | null;
  gpuLoad: number | null;
  pollMs: number;
  source: string;
}

export interface Fan { id: number; rpm: number; min: number; max: number; name: string }
export interface SmcTemp { key: string; group: string; kind: string; index: number | null; celsius: number; name: string; hw: string }
export interface SensorStatus { platform: string; admin: boolean; driver: boolean; helper: string; error: string; sensorCount: number }
export interface SmcSensors { fans: Fan[]; temps: SmcTemp[]; supported: boolean }

export interface StressStatus {
  running: boolean;
  kind: string;
  threads: number;
  elapsedSecs: number;
  durationSecs: number;
  ops: number;
  opsPerSec: number;
  reason: string;
  diskWriteMbs: number;
  diskReadMbs: number;
  device: string;
  error: string;
}

export interface ProcInfo { pid: number; name: string; cpu: number; memory: number; status: string; runTime: number; exe: string }
export interface StartupItem {
  id: string;
  name: string;
  command: string;
  location: string;
  enabled: boolean;
  canToggle: boolean;
  canRemove: boolean;
}
export interface DetailNode { name: string; props: [string, string][]; children: DetailNode[] }
export interface Connection { process: string; pid: number; local: string; remote: string; state: string }
export interface SnapshotMeta { id: string; name: string; created: number }

export const api = {
  hardware: () => invoke<HardwareInfo>("hardware_info"),
  live: () => invoke<LiveStats>("live_stats"),
  processes: () => invoke<ProcInfo[]>("processes"),
  kill: (pid: number) => invoke<boolean>("kill_process", { pid }),
  startup: () => invoke<StartupItem[]>("startup_items"),
  startupToggle: (id: string, enabled: boolean) => invoke<void>("startup_set_enabled", { id, enabled }),
  startupRemove: (id: string) => invoke<void>("startup_remove", { id }),
  startupAdd: (path: string) => invoke<void>("startup_add", { path }),
  detailCategories: () => invoke<string[]>("detail_categories"),
  detailData: (category: string) => invoke<DetailNode[]>("detail_data", { category }),
  power: () => invoke<PowerStats | null>("power_stats"),
  smc: () => invoke<SmcSensors>("smc_sensors"),
  sensorStatus: () => invoke<SensorStatus>("sensor_status"),
  installDriver: () => invoke<string>("install_sensor_driver"),
  stressStart: (kind: string, threads: number, seconds: number) => invoke<void>("stress_start", { kind, threads, seconds }),
  stressStop: (reason?: string) => invoke<void>("stress_stop", { reason }),
  stressStatus: () => invoke<StressStatus>("stress_status"),
  connections: () => invoke<Connection[]>("connections"),
  saveSnapshot: (name: string, data: HardwareInfo) => invoke<SnapshotMeta>("save_snapshot", { name, data: JSON.stringify(data) }),
  snapshots: () => invoke<SnapshotMeta[]>("list_snapshots"),
  loadSnapshot: async (id: string) => JSON.parse(await invoke<string>("load_snapshot", { id })) as HardwareInfo,
  deleteSnapshot: (id: string) => invoke<void>("delete_snapshot", { id }),
  saveFile: (path: string, content: string) => invoke<void>("save_text_file", { path, content }),
  windowEffect: (effect: string) => invoke<void>("set_window_effect", { effect }),
};
