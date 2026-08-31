import { useCallback, useEffect, useState } from "react";
import { startLayer, stopLayer, restartLayer, switchRuntime } from "./api";
import type { LayerStatus, RuntimeState } from "./types";

function activeState(layer: LayerStatus): RuntimeState {
  if (layer.active_runtime) {
    const rt = layer.runtimes.find((r) => r.name === layer.active_runtime);
    if (rt) return rt.state;
  }
  for (const rt of layer.runtimes) {
    if (rt.state !== "stopped" && rt.state !== "unknown") {
      return rt.state;
    }
  }
  return "stopped";
}

export function useKeyboardShortcuts(
  layers: LayerStatus[],
  onUpdate: (layer: LayerStatus) => void
) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const handleAction = useCallback(
    async (action: () => Promise<LayerStatus>) => {
      try {
        const updated = await action();
        onUpdate(updated);
      } catch {
        // Action errors are shown per-card via polling
      }
    },
    [onUpdate]
  );

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      // Ignore if user is typing in an input
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      ) {
        return;
      }

      // Number keys 1-9: select a layer
      if (e.key >= "1" && e.key <= "9") {
        const idx = parseInt(e.key) - 1;
        if (idx < layers.length) {
          setSelectedIndex(idx);
        }
        return;
      }

      // Escape: deselect
      if (e.key === "Escape") {
        setSelectedIndex(null);
        return;
      }

      // Action keys only work when a layer is selected
      if (selectedIndex === null || selectedIndex >= layers.length) return;

      const layer = layers[selectedIndex];
      const state = activeState(layer);
      const isStopped = state === "stopped" || state === "unknown";
      const isRunning = state === "running";

      if (e.key === "s") {
        if (isStopped) {
          handleAction(() => startLayer(layer.name));
        } else {
          handleAction(() => stopLayer(layer.name));
        }
        setSelectedIndex(null);
        return;
      }

      if (e.key === "r" && isRunning) {
        handleAction(() => restartLayer(layer.name));
        setSelectedIndex(null);
        return;
      }

      if (e.key === "Tab" && layer.runtimes.length > 1) {
        e.preventDefault();
        const activeIdx = layer.active_runtime
          ? layer.runtimes.findIndex((r) => r.name === layer.active_runtime)
          : -1;
        const nextIdx = (activeIdx + 1) % layer.runtimes.length;
        const nextRuntime = layer.runtimes[nextIdx].name;

        if (isStopped) {
          handleAction(() => startLayer(layer.name, nextRuntime));
        } else {
          handleAction(() => switchRuntime(layer.name, nextRuntime));
        }
        setSelectedIndex(null);
        return;
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [layers, selectedIndex, handleAction]);

  return { selectedIndex };
}
