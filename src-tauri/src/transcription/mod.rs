//! Transcrição plugável. Provedor `OpenAiCompatible` (multipart, Bearer) —
//! cobre Groq/OpenAI Whisper e qualquer endpoint compatível. Default = Groq.
//! Retorna segmentos com timestamp (via `verbose_json`) para intercalar faixas.
//! A chave vem do keychain (nunca daqui). Ver docs/MINIMAX.md.

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Config não-secreta do provedor (persistida em SQLite). A chave fica no keychain.
#[derive(Serialize, Deserialize, Clone)]
pub struct TranscriptionConfig {
    /// URL completa do endpoint de transcrição.
    pub endpoint_url: String,
    /// Nome do modelo enviado no campo `model`.
    pub model: String,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        // Groq Whisper (OpenAI-compatível, free tier). MiniMax NÃO tem STT.
        Self {
            endpoint_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3-turbo".to_string(),
        }
    }
}

/// Um trecho transcrito com o instante de início (segundos).
pub struct TranscriptSegment {
    pub start: f64,
    pub text: String,
}

pub trait Transcriber {
    /// Transcreve o arquivo no idioma indicado (ex.: "pt"), em segmentos.
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Vec<TranscriptSegment>>;
}

/// Valida a chave/endpoint sem enviar áudio: GET `<base>/models` (espera 200).
/// Deriva a base trocando `/audio/transcriptions` por `/models`.
pub fn test_key(endpoint_url: &str, api_key: &str) -> Result<()> {
    let models_url = if endpoint_url.contains("/audio/transcriptions") {
        endpoint_url.replace("/audio/transcriptions", "/models")
    } else {
        endpoint_url.to_string()
    };
    let resp = crate::net::client(20)
        .get(&models_url)
        .bearer_auth(api_key)
        .send()
        .map_err(|e| anyhow!("falha na conexão: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().unwrap_or_default();
    bail!("provedor retornou {status}: {body}");
}

/// Muletas curtas que o Whisper costuma inventar em trechos sem fala. Só são
/// descartadas junto de um indício de silêncio — "E aí" pode ser fala real.
const FILLER_HALLUCINATIONS: &[&str] = &[
    "e aí",
    "e ai",
    "aí",
    "obrigado",
    "obrigada",
    "muito obrigado",
    "valeu",
    "tchau",
    "até logo",
    "até mais",
    "inscreva-se",
    "inscreva-se no canal",
    "legendas pela comunidade amara.org",
    "legendado pela comunidade amara.org",
    "thanks for watching",
    "thank you",
    "thank you.",
    "subscribe",
    "bye",
    "you",
];

/// Texto sem pontuação nas bordas e em minúsculas, para casar com a lista.
fn normalized(text: &str) -> String {
    text.trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c == '…' || c.is_whitespace())
        .to_lowercase()
}

/// Heurística anti-alucinação do Whisper. Numa reunião, a faixa de cada lado
/// fica muda enquanto o outro fala, e o modelo "preenche" o silêncio com
/// muletas. O `verbose_json` traz, por segmento, `no_speech_prob` (chance de
/// não haver fala) e `avg_logprob` (confiança média) — descarta o que combina
/// indício de silêncio com baixa confiança. Provedores que não mandam esses
/// campos caem nos defaults neutros e nada é filtrado.
fn is_hallucination(text: &str, no_speech_prob: f64, avg_logprob: f64) -> bool {
    if no_speech_prob > 0.6 && avg_logprob < -0.4 {
        return true;
    }
    no_speech_prob > 0.5 && FILLER_HALLUCINATIONS.contains(&normalized(text).as_str())
}

#[cfg(test)]
mod tests {
    use super::is_hallucination;

    #[test]
    fn descarta_silencio_com_baixa_confianca() {
        assert!(is_hallucination("E aí", 0.9, -0.8));
        assert!(is_hallucination("Legendas pela comunidade Amara.org", 0.95, -0.6));
    }

    #[test]
    fn descarta_muleta_curta_em_trecho_mudo() {
        // Confiança razoável, mas forte indício de silêncio + frase-muleta.
        assert!(is_hallucination("E aí.", 0.7, -0.2));
        assert!(is_hallucination("Obrigado!", 0.55, -0.1));
    }

    #[test]
    fn mantem_fala_real() {
        // Mesma frase, mas com fala detectada: não pode sumir.
        assert!(!is_hallucination("E aí", 0.05, -0.2));
        // Frase longa com confiança boa, mesmo com no_speech_prob alto.
        assert!(!is_hallucination("vamos fechar o valuation na semana que vem", 0.8, -0.2));
    }

    #[test]
    fn sem_campos_do_provedor_nao_filtra() {
        // Defaults neutros (0.0/0.0) quando o provedor não manda as métricas.
        assert!(!is_hallucination("E aí", 0.0, 0.0));
    }
}

/// Provedor multipart compatível com a API OpenAI de transcrição.
#[derive(Clone)]
pub struct OpenAiCompatible {
    pub endpoint_url: String,
    pub model: String,
    pub api_key: String,
}

impl Transcriber for OpenAiCompatible {
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Vec<TranscriptSegment>> {
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "verbose_json")
            .file("file", audio_path)
            .map_err(|e| anyhow!("falha ao anexar o áudio: {e}"))?;

        let resp = crate::net::client(180)
            .post(&self.endpoint_url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|e| anyhow!("falha na requisição ao provedor: {e}"))?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        if !status.is_success() {
            bail!("provedor retornou {status}: {body}");
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow!("resposta não-JSON ({e}): {body}"))?;

        // verbose_json: array "segments" com start/text.
        if let Some(segs) = json.get("segments").and_then(|s| s.as_array()) {
            let mut out = Vec::new();
            let mut dropped = 0usize;
            for s in segs {
                let start = s.get("start").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let text = s
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let no_speech = s.get("no_speech_prob").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let logprob = s.get("avg_logprob").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if text.is_empty() {
                    continue;
                }
                if is_hallucination(&text, no_speech, logprob) {
                    dropped += 1;
                    continue;
                }
                out.push(TranscriptSegment { start, text });
            }
            // Só descartou = faixa era silêncio puro; devolve vazio (não é erro).
            if !out.is_empty() || dropped > 0 {
                return Ok(out);
            }
        }

        // Fallback: só o campo `text` como um único segmento.
        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            bail!("resposta sem texto: {body}");
        }
        Ok(vec![TranscriptSegment { start: 0.0, text }])
    }
}
