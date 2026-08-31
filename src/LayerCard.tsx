import { useState } from "react";
import { startLayer, stopLayer, restartLayer, switchRuntime } from "./api";
import type { LayerStatus, RuntimeState } from "./types";

interface Props {
  layer: LayerStatus;
  index: number;
  selected: boolean;
  onUpdate: (layer: LayerStatus) => void;
  onMoveUp?: () => void;
  onMoveDown?: () => void;
}

function stateColor(state: RuntimeState): string {
  switch (state) {
    case "running":
      return "var(--green)";
    case "starting":
    case "stopping":
      return "var(--yellow)";
    case "crashed":
    case "error":
      return "var(--red)";
    default:
      return "var(--muted)";
  }
}

function stateLabel(state: RuntimeState): string {
  return state.charAt(0).toUpperCase() + state.slice(1);
}

function activeState(layer: LayerStatus): RuntimeState {
  if (layer.active_runtime) {
    const rt = layer.runtimes.find((r) => r.name === layer.active_runtime);
    if (rt) return rt.state;
  }
  // Check if any runtime is in a non-stopped state
  for (const rt of layer.runtimes) {
    if (rt.state !== "stopped" && rt.state !== "unknown") {
      return rt.state;
    }
  }
  return "stopped";
}

export function LayerCard({ layer, index, selected, onUpdate, onMoveUp, onMoveDown }: Props) {
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const state = activeState(layer);
  const isRunning = state === "running";
  const isStopped = state === "stopped" || state === "unknown";
  const disabled = busy || layer.transitioning;

  async function handleAction(action: () => Promise<LayerStatus>) {
    setBusy(true);
    setActionError(null);
    try {
      const updated = await action();
      onUpdate(updated);
    } catch (e) {
      setActionError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleStart(runtimeName?: string) {
    await handleAction(() => startLayer(layer.name, runtimeName));
  }

  async function handleStop() {
    await handleAction(() => stopLayer(layer.name));
  }

  async function handleRestart() {
    await handleAction(() => restartLayer(layer.name));
  }

  async function handleSwitch(runtimeName: string) {
    await handleAction(() => switchRuntime(layer.name, runtimeName));
  }

  const isMultiRuntime = layer.runtimes.length > 1;
  const showNumber = index < 9;

  return (
    <div className={`layer-card${selected ? " layer-card-selected" : ""}`}>
      <div className="layer-header">
        <span className="layer-name">
          {showNumber && <span className="key-badge">{index + 1}</span>}
          {layer.name}
        </span>
        <div className="reorder-btns">
          <button
            className="reorder-btn"
            disabled={!onMoveUp}
            onClick={onMoveUp}
            title="Move up"
          >
            &#9650;
          </button>
          <button
            className="reorder-btn"
            disabled={!onMoveDown}
            onClick={onMoveDown}
            title="Move down"
          >
            &#9660;
          </button>
        </div>
      </div>

      {isMultiRuntime && (
        <RuntimeToggle
          layer={layer}
          disabled={disabled}
          selected={selected}
          onSwitch={handleSwitch}
          onStart={handleStart}
        />
      )}

      <div className="layer-status">
        <span
          className="status-dot"
          style={{ backgroundColor: stateColor(state) }}
        />
        <span className="status-label">
          {layer.transition_message ?? stateLabel(state)}
        </span>
      </div>

      {actionError && <div className="error-text">{actionError}</div>}

      {layer.runtimes.map((rt) =>
        rt.error ? (
          <div key={rt.name} className="error-text">
            {rt.name}: {rt.error}
          </div>
        ) : null
      )}

      <div className="layer-actions">
        {isStopped && !isMultiRuntime && (
          <button
            className="btn"
            disabled={disabled}
            onClick={() => handleStart()}
          >
            {selected && <span className="key-hint">[s]</span>}
            Start
          </button>
        )}
        {isRunning && (
          <button className="btn" disabled={disabled} onClick={handleRestart}>
            {selected && <span className="key-hint">[r]</span>}
            Restart
          </button>
        )}
        {!isStopped && (
          <button
            className="btn btn-stop"
            disabled={disabled}
            onClick={handleStop}
          >
            {selected && <span className="key-hint">[s]</span>}
            Stop
          </button>
        )}
      </div>
    </div>
  );
}

function RuntimeToggle({
  layer,
  disabled,
  selected,
  onSwitch,
  onStart,
}: {
  layer: LayerStatus;
  disabled: boolean;
  selected: boolean;
  onSwitch: (name: string) => void;
  onStart: (name: string) => void;
}) {
  const runtimes = layer.runtimes;
  const activeIdx = layer.active_runtime
    ? runtimes.findIndex((r) => r.name === layer.active_runtime)
    : -1;
  const state = activeState(layer);
  const isStopped = state === "stopped" || state === "unknown";

  return (
    <div className="runtime-toggle">
      {selected && <span className="key-hint key-hint-toggle">[tab]</span>}
      {runtimes.map((rt, idx) => {
        const isActive = idx === activeIdx;
        return (
          <button
            key={rt.name}
            className={`runtime-btn ${isActive ? "runtime-active" : ""}`}
            disabled={disabled || isActive}
            onClick={() => {
              if (isStopped) {
                onStart(rt.name);
              } else {
                onSwitch(rt.name);
              }
            }}
          >
            {rt.name}
          </button>
        );
      })}
    </div>
  );
}
