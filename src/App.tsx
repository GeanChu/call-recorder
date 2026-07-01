import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import "./App.css";

type Tab = "gravar" | "gravacoes" | "transcricao" | "config";

type Recording = {
  id: string;
  path: string;
  system_path: string | null;
  created_at: number;
  duration_s: number;
  size_bytes: number;
};

type Transcript = {
  recording_id: string;
  language: string;
  text: string;
  created_at: number;
};

type Settings = {
  default_language: string;
  endpoint_url: string;
  model: string;
  has_api_key: boolean;
  summary_endpoint_url: string;
  summary_model: string;
  has_summary_key: boolean;
};

type Summary = {
  recording_id: string;
  text: string;
  created_at: number;
};

const TABS: { id: Tab; label: string }[] = [
  { id: "gravar", label: "Gravar" },
  { id: "gravacoes", label: "Gravações" },
  { id: "transcricao", label: "Transcrição" },
  { id: "config", label: "Configurações" },
];

const LANGUAGES: { code: string; label: string }[] = [
  { code: "pt", label: "Português (BR)" },
  { code: "en", label: "Inglês" },
  { code: "es", label: "Espanhol" },
  { code: "fr", label: "Francês" },
  { code: "de", label: "Alemão" },
  { code: "it", label: "Italiano" },
];

function App() {
  const [tab, setTab] = useState<Tab>("gravar");
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);

  const refreshRecordings = useCallback(async () => {
    try {
      setRecordings(await invoke<Recording[]>("list_recordings"));
    } catch {
      /* ignore */
    }
  }, []);

  const refreshSettings = useCallback(async () => {
    try {
      setSettings(await invoke<Settings>("get_settings"));
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    refreshRecordings();
    refreshSettings();
  }, [refreshRecordings, refreshSettings]);

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
        {tab === "gravar" && <RecordScreen onFinished={refreshRecordings} />}
        {tab === "gravacoes" && (
          <RecordingsScreen recordings={recordings} onChanged={refreshRecordings} />
        )}
        {tab === "transcricao" && (
          <TranscriptionScreen
            recordings={recordings}
            defaultLanguage={settings?.default_language ?? "pt"}
            hasApiKey={settings?.has_api_key ?? false}
            hasSummaryKey={settings?.has_summary_key ?? false}
          />
        )}
        {tab === "config" && <ConfigScreen settings={settings} onSaved={refreshSettings} />}
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
        Grava microfone + áudio do sistema (Windows) em faixas separadas (Opus, leve). A transcrição
        rotula "Você" (mic) e "Participantes" (sistema). No Linux/macOS o áudio do sistema chega depois.
      </p>
      {error && <p className="error">{error}</p>}
    </section>
  );
}

function RecordingsScreen({
  recordings,
  onChanged,
}: {
  recordings: Recording[];
  onChanged: () => void;
}) {
  const [playing, setPlaying] = useState<string | null>(null);

  async function remove(id: string) {
    if (!window.confirm("Apagar esta gravação e sua transcrição? Não dá pra desfazer.")) return;
    try {
      await invoke("delete_recording", { recordingId: id });
      if (playing === id) setPlaying(null);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  }

  function mixSrc(micPath: string): string {
    return convertFileSrc(micPath.replace(/mic\.webm$/, "recording.webm"));
  }

  return (
    <section className="panel">
      <h2>Gravações</h2>
      {recordings.length === 0 ? (
        <p className="hint">Nenhuma gravação ainda. Grave na aba Gravar.</p>
      ) : (
        <ul className="rec-list">
          {recordings.map((r) => (
            <li key={r.id}>
              <div className="rec-row">
                <div>
                  <strong>{formatDate(r.created_at)}</strong> — {formatTime(Math.round(r.duration_s))} ·{" "}
                  {formatSize(r.size_bytes)}
                </div>
                <div className="rec-actions">
                  <button
                    className="play-btn"
                    onClick={() => setPlaying(playing === r.id ? null : r.id)}
                  >
                    {playing === r.id ? "Fechar" : "▶ Play"}
                  </button>
                  <button className="del-btn" onClick={() => remove(r.id)}>
                    Apagar
                  </button>
                </div>
              </div>
              {playing === r.id && (
                <audio className="player" controls autoPlay src={mixSrc(r.path)} />
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function TranscriptionScreen({
  recordings,
  defaultLanguage,
  hasApiKey,
  hasSummaryKey,
}: {
  recordings: Recording[];
  defaultLanguage: string;
  hasApiKey: boolean;
  hasSummaryKey: boolean;
}) {
  const [selectedId, setSelectedId] = useState("");
  const [language, setLanguage] = useState(defaultLanguage);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [summary, setSummary] = useState("");
  const [sumBusy, setSumBusy] = useState(false);
  const [sumError, setSumError] = useState<string | null>(null);
  const [sumCopied, setSumCopied] = useState(false);

  useEffect(() => {
    if (!selectedId && recordings.length > 0) setSelectedId(recordings[0].id);
  }, [recordings, selectedId]);

  useEffect(() => {
    setText("");
    setSummary("");
    setError(null);
    setSumError(null);
    if (!selectedId) return;
    invoke<Transcript | null>("get_transcript", { recordingId: selectedId })
      .then((t) => {
        if (t) {
          setText(t.text);
          setLanguage(t.language);
        }
      })
      .catch(() => {});
    invoke<Summary | null>("get_summary", { recordingId: selectedId })
      .then((s) => {
        if (s) setSummary(s.text);
      })
      .catch(() => {});
  }, [selectedId]);

  async function run() {
    setError(null);
    setBusy(true);
    try {
      const t = await invoke<Transcript>("transcribe", { recordingId: selectedId, language });
      setText(t.text);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  }

  async function makeSummary() {
    setSumError(null);
    setSumBusy(true);
    try {
      const s = await invoke<Summary>("generate_summary", { recordingId: selectedId });
      setSummary(s.text);
    } catch (e) {
      setSumError(String(e));
    } finally {
      setSumBusy(false);
    }
  }

  async function copySummary() {
    await navigator.clipboard.writeText(summary);
    setSumCopied(true);
    window.setTimeout(() => setSumCopied(false), 1500);
  }

  return (
    <section className="panel">
      <h2>Transcrição</h2>
      {recordings.length === 0 ? (
        <p className="hint">Grave algo primeiro na aba Gravar.</p>
      ) : (
        <>
          <div className="form-row">
            <label>Gravação</label>
            <select value={selectedId} onChange={(e) => setSelectedId(e.target.value)}>
              {recordings.map((r) => (
                <option key={r.id} value={r.id}>
                  {formatDate(r.created_at)} — {formatTime(Math.round(r.duration_s))}
                </option>
              ))}
            </select>
          </div>

          <div className="form-row">
            <label>Idioma</label>
            <select value={language} onChange={(e) => setLanguage(e.target.value)}>
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </div>

          {!hasApiKey && (
            <p className="hint">Configure a chave da API em Configurações antes de transcrever.</p>
          )}

          <div className="actions">
            <button onClick={run} disabled={busy || !selectedId}>
              {busy ? "Transcrevendo..." : "Transcrever"}
            </button>
            {text && (
              <button className="secondary" onClick={copy}>
                {copied ? "Copiado!" : "Copiar"}
              </button>
            )}
          </div>

          {error && <p className="error">{error}</p>}
          {text && <textarea className="transcript" readOnly value={text} />}

          {text && (
            <div className="summary-block">
              <h3>Resumo (opcional)</h3>
              {!hasSummaryKey && (
                <p className="hint">Configure a chave do Resumo (MiniMax) em Configurações.</p>
              )}
              <div className="actions">
                <button onClick={makeSummary} disabled={sumBusy || !hasSummaryKey}>
                  {sumBusy ? "Resumindo..." : summary ? "Refazer resumo" : "Gerar resumo"}
                </button>
                {summary && (
                  <button className="secondary" onClick={copySummary}>
                    {sumCopied ? "Copiado!" : "Copiar resumo"}
                  </button>
                )}
              </div>
              {sumError && <p className="error">{sumError}</p>}
              {summary && <textarea className="transcript" readOnly value={summary} />}
            </div>
          )}
        </>
      )}
    </section>
  );
}

function ConfigScreen({
  settings,
  onSaved,
}: {
  settings: Settings | null;
  onSaved: () => void;
}) {
  const [defaultLanguage, setDefaultLanguage] = useState("pt");
  const [endpointUrl, setEndpointUrl] = useState("");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [summaryEndpointUrl, setSummaryEndpointUrl] = useState("");
  const [summaryModel, setSummaryModel] = useState("");
  const [summaryKey, setSummaryKey] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (settings) {
      setDefaultLanguage(settings.default_language);
      setEndpointUrl(settings.endpoint_url);
      setModel(settings.model);
      setSummaryEndpointUrl(settings.summary_endpoint_url);
      setSummaryModel(settings.summary_model);
    }
  }, [settings]);

  async function save() {
    setError(null);
    setMsg(null);
    try {
      await invoke("save_settings", {
        defaultLanguage,
        endpointUrl,
        model,
        summaryEndpointUrl,
        summaryModel,
      });
      if (apiKey.trim()) {
        await invoke("set_api_key", { key: apiKey });
        setApiKey("");
      }
      if (summaryKey.trim()) {
        await invoke("set_summary_key", { key: summaryKey });
        setSummaryKey("");
      }
      setMsg("Configurações salvas.");
      onSaved();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="panel">
      <h2>Configurações</h2>

      <div className="form-row">
        <label>Idioma padrão</label>
        <select value={defaultLanguage} onChange={(e) => setDefaultLanguage(e.target.value)}>
          {LANGUAGES.map((l) => (
            <option key={l.code} value={l.code}>
              {l.label}
            </option>
          ))}
        </select>
      </div>

      <h3 className="cfg-section">Transcrição (Groq / Whisper)</h3>
      <p className="hint">Converte o áudio da reunião em texto.</p>
      <div className="form-row">
        <label>Endpoint</label>
        <input
          value={endpointUrl}
          onChange={(e) => setEndpointUrl(e.target.value)}
          placeholder="https://api.groq.com/openai/v1/audio/transcriptions"
        />
      </div>
      <div className="form-row">
        <label>Modelo</label>
        <input
          value={model}
          onChange={(e) => setModel(e.target.value)}
          placeholder="whisper-large-v3-turbo"
        />
      </div>
      <div className="form-row">
        <label>Chave da API</label>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder={settings?.has_api_key ? "•••••• (configurada)" : "cole a chave da transcrição"}
        />
      </div>

      <h3 className="cfg-section">Resumo (MiniMax-M3) — opcional</h3>
      <p className="hint">Gera um resumo da reunião a partir da transcrição. Usa a chave sk-cp da MiniMax.</p>
      <div className="form-row">
        <label>Endpoint</label>
        <input
          value={summaryEndpointUrl}
          onChange={(e) => setSummaryEndpointUrl(e.target.value)}
          placeholder="https://api.minimax.io/v1/chat/completions"
        />
      </div>
      <div className="form-row">
        <label>Modelo</label>
        <input
          value={summaryModel}
          onChange={(e) => setSummaryModel(e.target.value)}
          placeholder="MiniMax-M3"
        />
      </div>
      <div className="form-row">
        <label>Chave da API (sk-cp)</label>
        <input
          type="password"
          value={summaryKey}
          onChange={(e) => setSummaryKey(e.target.value)}
          placeholder={settings?.has_summary_key ? "•••••• (configurada)" : "cole a chave do resumo"}
        />
      </div>

      <p className="hint">As chaves ficam no keychain do sistema, nunca em texto puro.</p>

      <div className="actions">
        <button onClick={save}>Salvar</button>
      </div>

      {msg && <p className="ok">{msg}</p>}
      {error && <p className="error">{error}</p>}
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
