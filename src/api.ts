import { invoke } from "@tauri-apps/api/core";
import type { LayerStatus } from "./types";

export async function getLayers(): Promise<LayerStatus[]> {
  return invoke<LayerStatus[]>("get_layers");
}

export async function startLayer(
  layerName: string,
  runtimeName?: string
): Promise<LayerStatus> {
  return invoke<LayerStatus>("start_layer", {
    layerName,
    runtimeName: runtimeName ?? null,
  });
}

export async function stopLayer(layerName: string): Promise<LayerStatus> {
  return invoke<LayerStatus>("stop_layer", { layerName });
}

export async function restartLayer(layerName: string): Promise<LayerStatus> {
  return invoke<LayerStatus>("restart_layer", { layerName });
}

export async function buildLayer(layerName: string): Promise<LayerStatus> {
  return invoke<LayerStatus>("build_layer", { layerName });
}

export async function switchRuntime(
  layerName: string,
  runtimeName: string
): Promise<LayerStatus> {
  return invoke<LayerStatus>("switch_runtime", { layerName, runtimeName });
}

export async function openConfigDir(): Promise<string> {
  return invoke<string>("open_config_dir");
}

export async function createSampleConfig(): Promise<string> {
  return invoke<string>("create_sample_config");
}

export async function reloadConfig(): Promise<LayerStatus[]> {
  return invoke<LayerStatus[]>("reload_config");
}
