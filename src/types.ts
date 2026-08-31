export type RuntimeState =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "building"
  | "crashed"
  | "error"
  | "unknown";

export interface RuntimeStatus {
  name: string;
  runtime_type: string;
  state: RuntimeState;
  error?: string;
  pid?: number;
  has_build: boolean;
}

export interface LayerStatus {
  name: string;
  runtimes: RuntimeStatus[];
  active_runtime: string | null;
  transitioning: boolean;
  transition_message?: string;
}
