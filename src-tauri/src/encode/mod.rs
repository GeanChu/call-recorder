//! Mix + encode via ffmpeg. Mistura mic + áudio do sistema numa faixa Opus leve.
//!
//! O caminho do `ffmpeg` é resolvido por quem chama: em produção o binário vem
//! empacotado como resource; em dev cai no PATH/`CALLREC_FFMPEG`.

use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Result};

/// Duração de um arquivo de áudio em segundos, lendo o cabeçalho via ffmpeg.
/// `ffmpeg -i <arquivo>` sai com erro (sem output) mas imprime "Duration:".
pub fn probe_duration(ffmpeg: &str, path: &str) -> Option<f64> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-hide_banner").arg("-i").arg(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    let idx = stderr.find("Duration:")? + "Duration:".len();
    let rest = stderr[idx..].trim_start();
    let hms = rest.split(',').next()?.trim(); // "HH:MM:SS.ss"
    let mut parts = hms.split(':');
    let h: f64 = parts.next()?.trim().parse().ok()?;
    let m: f64 = parts.next()?.trim().parse().ok()?;
    let s: f64 = parts.next()?.trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Mistura `mic` (+ `system` se houver) numa faixa Opus mono ~32 kbps, 16 kHz.
/// O container vem da extensão de `out` (usamos `.webm`, aceito pela MiniMax).
pub fn mix_to_opus(ffmpeg: &str, mic: &str, system: Option<&str>, out: &Path) -> Result<()> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-y").arg("-i").arg(mic);

    if let Some(sys) = system {
        cmd.arg("-i").arg(sys);
        cmd.arg("-filter_complex")
            .arg("amix=inputs=2:duration=longest:normalize=0");
    }

    cmd.arg("-ac").arg("1").arg("-ar").arg("16000");
    cmd.arg("-c:a").arg("libopus").arg("-b:a").arg("32k");
    cmd.arg(out);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| anyhow!("falha ao executar ffmpeg ('{ffmpeg}'): {e}. Instale o ffmpeg ou defina CALLREC_FFMPEG."))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.lines().last().unwrap_or("erro desconhecido");
        bail!("ffmpeg falhou: {last}");
    }
    Ok(())
}

/// Transcodifica `src` (webm/opus) para `out`. O codec vem da extensão de `out`
/// (wav = PCM, mp3 = libmp3lame, ogg = libvorbis). Usado no "Exportar áudio".
pub fn transcode(ffmpeg: &str, src: &Path, out: &Path) -> Result<()> {
    let ext = out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-y").arg("-i").arg(src);
    match ext.as_str() {
        "mp3" => {
            cmd.arg("-c:a").arg("libmp3lame").arg("-q:a").arg("2");
        }
        "ogg" => {
            // Opus-em-Ogg: libopus está garantido nos 3 builds (usado no mix),
            // ao contrário do libvorbis, que pode faltar no ffmpeg estático do Linux.
            cmd.arg("-c:a").arg("libopus").arg("-b:a").arg("64k");
        }
        "wav" => {
            cmd.arg("-c:a").arg("pcm_s16le");
        }
        other => bail!("formato não suportado: {other}"),
    }
    cmd.arg(out);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| anyhow!("falha ao executar ffmpeg ('{ffmpeg}'): {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.lines().last().unwrap_or("erro desconhecido");
        bail!("ffmpeg falhou: {last}");
    }
    Ok(())
}
