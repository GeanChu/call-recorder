import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Tab = "gravar" | "gravacoes" | "transcricao" | "config";

type Recording = {
  id: string;
  path: string;
  created_at: number;
  duration_s: number;
  size_bytes: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "gravar", label: "Gravar" },
  { id: "gravacoes", label: "Gravações" },
  { id: "transcricao", label: "Transcrição" },
  { id: "config", label: "Configurações" },
];

function App() {
  const [tab, setTab] = useState<Tab>("gravar");
  const [recordings, setRecordings] = useState<Recording[]>([]);

  const refresh = useCallback(async () => {
    try {
      setRecordings(await invoke<Recording[]>("list_recordings"));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="app">
      <nav className="sidebar">
        <h1 className="brand">Call Recorder</h1>
        {TABS.map((t) => (
          <button
            key={t.id}
            className={tab === t.id ? "nav-item active" : "nav-item"}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <main className="content">
        {tab === "gravar" && <RecordScreen onFinished={refresh} />}
        {tab === "gravacoes" && <RecordingsScreen recordings={recordings} />}
        {tab === "transcricao" && (
          <Placeholder title="Transcrição" hint="Seleção de idioma (padrão pt-BR), texto e copiar (PR5)." />
        )}
        {tab === "config" && (
          <Placeholder title="Configurações" hint="Idioma padrão, chave da API e 'gravar todos' (PR6)." />
        )}
      </main>
    </div>
  );
}

function RecordScreen({ onFinished }: { onFinished: () => void }) {
  const [recording, setRecording] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [level, setLevel] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const timers = useRef<number[]>([]);

  useEffect(() => {
    invoke<boolean>("is_recording").then(setRecording).catch(() => {});
    return () => timers.current.forEach(clearInterval);
  }, []);

  function clearTimers() {
    timers.current.forEach(clearInterval);
    timers.current = [];
  }

  async function start() {
    setError(null);
    try {
      await invoke("start_recording");
      setRecording(true);
      setElapsed(0);
      const t1 = window.setInterval(() => setElapsed((e) => e + 1), 1000);
      const t2 = window.setInterval(async () => {
        try {
          setLevel(await invoke<number>("recording_level"));
        } catch {
          /* ignore */
        }
      }, 100);
      timers.current = [t1, t2];
    } catch (e) {
      setError(String(e));
    }
  }

  async function stop() {
    clearTimers();
    setLevel(0);
    setBusy(true);
    try {
      await invoke<Recording>("stop_recording");
      onFinished();
    } catch (e) {
      setError(String(e));
    } finally {
      setRecording(false);
      setBusy(false);
    }
  }

  return (
    <section className="panel record">
      <h2>Gravar</h2>
      <button
        className={recording ? "rec-btn stop" : "rec-btn"}
        onClick={recording ? stop : start}
        disabled={busy}
      >
        {busy ? "Processando..." : recording ? "Parar" : "Gravar"}
      </button>

      {recording && (
        <div className="meters">
          <div className="timer">{formatTime(elapsed)}</div>
          <div className="level-bar">
            <div className="level-fill" style={{ width: `${Math.min(level * 100, 100)}%` }} />
          </div>
        </div>
      )}

      <p className="hint">
        Grava microfone + áudio do sistema (Windows) e salva como Opus (.ogg) leve. No Linux/macOS o
        áudio do sistema chega depois.
      </p>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

function RecordingsScreen({ recordings }: { recordings: Recording[] }) {
  return (
    <section className="panel">
      <h2>Gravações</h2>
      {recordings.length === 0 ? (
        <p className="hint">Nenhuma gravação ainda. Grave na aba Gravar.</p>
      ) : (
        <ul className="rec-list">
          {recordings.map((r) => (
            <li key={r.id}>
              <strong>{formatDate(r.created_at)}</strong> — {formatTime(Math.round(r.duration_s))} ·{" "}
              {formatSize(r.size_bytes)}
              <div className="path">{r.path}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Placeholder({ title, hint }: { title: string; hint: string }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p className="hint">{hint}</p>
    </section>
  );
}

function formatTime(s: number): string {
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function formatDate(ms: number): string {
  return new Date(ms).toLocaleString("pt-BR");
}

export default App;
