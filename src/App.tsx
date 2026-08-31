import { useCallback, useEffect, useRef, useState } from "react";
import { getLayers, openConfigDir, createSampleConfig, reloadConfig } from "./api";
import { LayerCard } from "./LayerCard";
import type { LayerStatus } from "./types";
import logoSvg from "./logo.svg";

const ORDER_KEY = "cmdr-layer-order";

function loadOrder(): string[] {
  try {
    const raw = localStorage.getItem(ORDER_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return [];
}

function saveOrder(names: string[]) {
  localStorage.setItem(ORDER_KEY, JSON.stringify(names));
}

function applyOrder(layers: LayerStatus[], order: string[]): LayerStatus[] {
  if (order.length === 0) return layers;
  const map = new Map(layers.map((l) => [l.name, l]));
  const ordered: LayerStatus[] = [];
  for (const name of order) {
    const layer = map.get(name);
    if (layer) {
      ordered.push(layer);
      map.delete(name);
    }
  }
  // Append any layers not in the saved order (new layers)
  for (const layer of map.values()) {
    ordered.push(layer);
  }
  return ordered;
}

export function App() {
  const [layers, setLayers] = useState<LayerStatus[]>([]);
  const [order, setOrder] = useState<string[]>(loadOrder);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined);

  const refresh = useCallback(async () => {
    try {
      const data = await getLayers();
      setLayers(data);
      setError(null);
      setLoading(false);
    } catch (e) {
      const msg = String(e);
      // Backend is still initializing -- stay in loading state
      if (msg.includes("Stack not initialized")) return;
      setError(msg);
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    intervalRef.current = setInterval(refresh, 2000);
    return () => clearInterval(intervalRef.current);
  }, [refresh]);

  const handleReload = useCallback(async () => {
    try {
      const data = await reloadConfig();
      setLayers(data);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleLayerUpdate = useCallback(
    (updated: LayerStatus) => {
      setLayers((prev) =>
        prev.map((l) => (l.name === updated.name ? updated : l))
      );
      // Refresh after a brief delay to catch async state changes
      setTimeout(refresh, 500);
    },
    [refresh]
  );

  const handleMove = useCallback(
    (name: string, direction: "up" | "down") => {
      const ordered = applyOrder(layers, order);
      const names = ordered.map((l) => l.name);
      const idx = names.indexOf(name);
      if (idx < 0) return;
      const targetIdx = direction === "up" ? idx - 1 : idx + 1;
      if (targetIdx < 0 || targetIdx >= names.length) return;
      [names[idx], names[targetIdx]] = [names[targetIdx], names[idx]];
      setOrder(names);
      saveOrder(names);
    },
    [layers, order]
  );

  if (loading) {
    return (
      <div className="loader">
        <img src={logoSvg} alt="" className="loader-logo" />
        <span className="loader-text">loading</span>
      </div>
    );
  }

  if (error && layers.length === 0) {
    return (
      <div className="container">
        <h1 className="title"><img src={logoSvg} alt="" className="logo" />cmdr</h1>
        <p className="error-text">{error}</p>
        <p className="muted">
          Place a <code>cmdr.yaml</code> in the config directory.
        </p>
        <div className="empty-actions">
          <button className="btn" onClick={async () => {
            try {
              await createSampleConfig();
            } catch (e) {
              setError(String(e));
            }
          }}>
            Generate Sample Config
          </button>
          <button className="btn" onClick={openConfigDir}>
            Open Config Folder
          </button>
          <button className="btn" onClick={handleReload}>
            Reload
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="container">
      <div className="header">
        <h1 className="title"><img src={logoSvg} alt="" className="logo" />cmdr</h1>
        <div className="header-actions">
          <button className="config-btn" onClick={handleReload} title="Reload configuration">
            Reload
          </button>
          <button className="config-btn" onClick={openConfigDir} title="Open config folder">
            Config
          </button>
        </div>
      </div>
      {(() => {
        const ordered = applyOrder(layers, order);
        return ordered.map((layer, idx) => (
          <LayerCard
            key={layer.name}
            layer={layer}
            onUpdate={handleLayerUpdate}
            onMoveUp={idx > 0 ? () => handleMove(layer.name, "up") : undefined}
            onMoveDown={idx < ordered.length - 1 ? () => handleMove(layer.name, "down") : undefined}
          />
        ));
      })()}
    </div>
  );
}
